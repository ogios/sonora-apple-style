mod arguments;
mod output;

use std::io::{self, Write as _};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context as _, Result, bail, ensure};
use arguments::{Arguments, Command};
use i18n::t;
use protocol::{Call, Client, ConnectionState, Snapshot};
use serde_json::{Value, json};

const LOAD_TIMEOUT: Duration = Duration::from_secs(120);

fn main() -> ExitCode {
    i18n::set(i18n::Language::detect());
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error)
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe) =>
        {
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("sonora-cli: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Handles local information without a server, then runs one IPC operation on Tokio.
fn run() -> Result<()> {
    let arguments = Arguments::parse()?;
    match arguments.command {
        Command::Help => return output::line(&t!("cli-help")),
        Command::Version => {
            return output::line(&format!("sonora-cli {}", env!("CARGO_PKG_VERSION")));
        }
        _ => {}
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("cannot start the CLI runtime")?;
    runtime.block_on(execute(arguments))
}

/// Uses the same acknowledged command boundary as the GUI without retrying mutations.
async fn execute(arguments: Arguments) -> Result<()> {
    let client = Client::connect()
        .await
        .with_context(|| t!("cli-connect-failed").to_string())?;
    let json = arguments.json;
    match arguments.command {
        Command::Status => {
            let snapshot = client.snapshot().await?;
            output::show("playback", object(&snapshot, "playback")?, json)
        }
        Command::Query(name) => {
            let snapshot = client.snapshot().await?;
            output::show(name, object(&snapshot, name)?, json)
        }
        Command::Watch => watch(&client, json).await,
        Command::Call(call) => {
            if call.object == "session" && call.method == "switch" {
                let snapshot = client.snapshot().await?;
                let providers = object(&snapshot, "session")?["providers"]
                    .as_array()
                    .context("the session is missing providers")?;
                ensure!(
                    providers
                        .iter()
                        .any(|provider| provider["slug"] == call.arguments[0]
                            && provider["stored"] == true),
                    "{}",
                    t!("cli-account-not-stored")
                );
            }
            invoke(&client, call).await?;
            output::accepted(json)
        }
        Command::Queue { method, index } => {
            let snapshot = client.snapshot().await?;
            let queue = object(&snapshot, "queue")?;
            if let Some(index) = index {
                let length = queue["upcoming"]
                    .as_array()
                    .context("the queue is missing upcoming tracks")?
                    .len();
                ensure!(index < length, "{}", t!("cli-invalid-index"));
            }
            let target = match method {
                "play_upcoming" => "playback",
                _ => "queue",
            };
            let mut call = Call::new(
                target,
                method,
                index.into_iter().map(|index| json!(index)).collect(),
            );
            call.revision = Some(
                queue["revision"]
                    .as_u64()
                    .context("the queue revision is missing")?,
            );
            invoke(&client, call).await?;
            output::accepted(json)
        }
        Command::Search(query) => {
            client.subscribe().await?;
            let name = "search:cli";
            invoke(&client, Call::new(name, "ask", vec![json!(query)])).await?;
            let page = wait_for(&client, name, |page| page["settled"] == true).await?;
            page_error(&page)?;
            output::show("search", &page, json)
        }
        Command::Track { id, enqueue } => {
            client.subscribe().await?;
            let name = "song:cli";
            invoke(&client, Call::new(name, "open", vec![json!(id)])).await?;
            let page = wait_for(&client, name, |page| page["loading"] == false).await?;
            page_error(&page)?;
            let track = page
                .pointer("/page/track")
                .filter(|track| !track.is_null())
                .ok_or_else(|| anyhow::anyhow!("{}", t!("cli-track-unavailable")))?;
            ensure!(track["playable"] == true, "{}", t!("cli-track-unavailable"));
            let method = match enqueue {
                true => "enqueue",
                false => "play",
            };
            invoke(&client, Call::new("playback", method, vec![track.clone()])).await?;
            output::accepted(json)
        }
        Command::Login { provider, method } => {
            let snapshot = client.subscribe().await?;
            let session = object(&snapshot, "session")?;
            ensure!(
                session["state"] != "Restoring" && session["state"].get("Authorizing").is_none(),
                "{}",
                t!("cli-account-busy")
            );
            let info = session["providers"]
                .as_array()
                .and_then(|providers| providers.iter().find(|info| info["slug"] == provider))
                .ok_or_else(|| anyhow::anyhow!("{}", t!("cli-unknown-provider")))?;
            let options = info["options"]
                .as_array()
                .context("the provider has no sign-in options")?;
            let method = match method.as_str() {
                "Default" => ["Default", "QrCode", "Secret", "Anonymous"]
                    .into_iter()
                    .map(|method| json!(method))
                    .find(|method| options.contains(method))
                    .ok_or_else(|| anyhow::anyhow!("{}", t!("cli-unsupported-auth")))?,
                _ => json!(method),
            };
            ensure!(options.contains(&method), "{}", t!("cli-unsupported-auth"));
            invoke(
                &client,
                Call::new("session", "sign_in_manually", vec![json!(provider), method]),
            )
            .await?;
            client.snapshot().await?;
            let session = wait_for(&client, "session", |session| {
                let state = &session["state"];
                state != "Restoring"
                    && state
                        .get("Authorizing")
                        .is_none_or(|prompt| !prompt.is_null())
            })
            .await?;
            authorization_error(&session, &provider)?;
            if session["state"]["Authorizing"].get("QrCode").is_some() {
                return follow_qr(&client, &provider, &session, json).await;
            }
            output::show("session", &session, json)
        }
        Command::Input => {
            let snapshot = client.snapshot().await?;
            let prompt = &object(&snapshot, "session")?["state"]["Authorizing"];
            ensure!(
                prompt == "Secret" || prompt.get("Accounts").is_some(),
                "{}",
                t!("cli-no-input-prompt")
            );
            let mut text = String::new();
            io::stdin()
                .read_line(&mut text)
                .context("cannot read sign-in input")?;
            let text = text.trim_end_matches(['\r', '\n']);
            ensure!(!text.is_empty(), "{}", t!("cli-empty-input"));
            invoke(
                &client,
                Call::new("session", "submit_input", vec![json!(text)]),
            )
            .await?;
            output::accepted(json)
        }
        Command::Help | Command::Version => unreachable!(),
    }
}

async fn invoke(client: &Client, call: Call) -> Result<Value> {
    client.call(call).await
}

/// Stays attached to a native QR approval and cancels that challenge on Ctrl+C.
async fn follow_qr(client: &Client, provider: &str, initial: &Value, json: bool) -> Result<()> {
    let url = initial["state"]["Authorizing"]["QrCode"]["url"]
        .as_str()
        .context("the QR authorization URL is missing")?;
    output::show("session", initial, json)?;
    io::stdout().flush().context("cannot flush CLI output")?;
    let mut previous = initial["state"].clone();
    let mut snapshots = client.snapshots();
    let mut connection = client.connection_state();
    loop {
        {
            let snapshot = snapshots.borrow_and_update();
            let session = object(&snapshot, "session")?;
            authorization_error(session, provider)?;
            let state = &session["state"];
            if state.get("SignedIn").is_some() && session["provider_slug"] == provider {
                return output::show("session", session, json);
            }
            let challenge = &state["Authorizing"]["QrCode"];
            let pending = session["providers"].as_array().is_some_and(|providers| {
                providers
                    .iter()
                    .any(|info| info["slug"] == provider && info["pending"] == true)
            });
            ensure!(
                pending && challenge["url"] == url,
                "{}",
                t!("cli-cancelled")
            );
            if state != &previous {
                match json {
                    true => output::show("session", session, true)?,
                    false => output::qr_status(challenge["scanned"] == true)?,
                }
                io::stdout().flush().context("cannot flush CLI output")?;
                previous = state.clone();
            }
        }
        if let ConnectionState::Disconnected(reason) = &*connection.borrow_and_update() {
            bail!("{reason}");
        }
        tokio::select! {
            result = snapshots.changed() => result.context("the service state stream closed")?,
            result = connection.changed() => result.context("the service connection closed")?,
            result = tokio::signal::ctrl_c() => {
                result.context("cannot listen for Ctrl+C")?;
                invoke(client, Call::new("session", "cancel_qr_sign_in", vec![json!(provider), json!(url)])).await?;
                bail!("{}", t!("cli-cancelled"));
            }
        }
    }
}

/// Reports failures retained on the requested provider even when a previous account resumes.
fn authorization_error(session: &Value, provider: &str) -> Result<()> {
    let error = session["providers"]
        .as_array()
        .and_then(|providers| providers.iter().find(|info| info["slug"] == provider))
        .and_then(|info| info["error"].as_object())
        .and_then(|failure| failure.get("summary"))
        .or_else(|| {
            session["state"]
                .get("Failed")
                .and_then(|failure| failure.get("summary"))
        });
    if let Some(summary) = error.and_then(Value::as_str) {
        bail!("{summary}");
    }
    Ok(())
}

fn object<'a>(snapshot: &'a Snapshot, name: &str) -> Result<&'a Value> {
    snapshot
        .objects
        .get(name)
        .with_context(|| format!("the service snapshot is missing {name}"))
}

/// Waits for provider work while observing disconnects and retaining the client's page lease.
async fn wait_for(client: &Client, name: &str, ready: impl Fn(&Value) -> bool) -> Result<Value> {
    let mut snapshots = client.snapshots();
    let mut connection = client.connection_state();
    tokio::time::timeout(LOAD_TIMEOUT, async {
        loop {
            {
                let snapshot = snapshots.borrow_and_update();
                if let Some(value) = snapshot.objects.get(name).filter(|value| ready(value)) {
                    return Ok(value.clone());
                }
            }
            if let ConnectionState::Disconnected(reason) = &*connection.borrow_and_update() {
                bail!("{reason}");
            }
            tokio::select! {
                result = snapshots.changed() => result.context("the service state stream closed")?,
                result = connection.changed() => result.context("the service connection closed")?,
                result = tokio::signal::ctrl_c() => {
                    result.context("cannot listen for Ctrl+C")?;
                    bail!("{}", t!("cli-cancelled"));
                }
            }
        }
    })
    .await
    .with_context(|| t!("cli-load-timeout").to_string())?
}

fn page_error(page: &Value) -> Result<()> {
    if let Some(error) = page["error"].as_str() {
        bail!("{error}");
    }
    Ok(())
}

/// Streams playback changes as text or newline-delimited JSON until Ctrl+C or disconnect.
async fn watch(client: &Client, json: bool) -> Result<()> {
    client.subscribe().await?;
    let mut snapshots = client.snapshots();
    let mut connection = client.connection_state();
    let mut previous = Value::Null;
    loop {
        {
            let snapshot = snapshots.borrow_and_update();
            let playback = object(&snapshot, "playback")?;
            if playback != &previous {
                output::show("playback", playback, json)?;
                io::stdout().flush().context("cannot flush CLI output")?;
                previous = playback.clone();
            }
        }
        if let ConnectionState::Disconnected(reason) = &*connection.borrow_and_update() {
            bail!("{reason}");
        }
        tokio::select! {
            result = snapshots.changed() => result.context("the service state stream closed")?,
            result = connection.changed() => result.context("the service connection closed")?,
            result = tokio::signal::ctrl_c() => return result.context("cannot listen for Ctrl+C"),
        }
    }
}
