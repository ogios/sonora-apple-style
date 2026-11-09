use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use service::{Io, Sonora};

fn main() -> ExitCode {
    if let Some(module) = music::drm::hosted() {
        return match music::drm::host(&module) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("server: cannot host the cdm: {error:#}");
                ExitCode::FAILURE
            }
        };
    }
    env_logger::Builder::from_env(
        env_logger::Env::default().filter_or("SONORA_SERVER_LOG", "warn,service=info"),
    )
    .init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("server: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Binds exclusivity before creating providers, then keeps service state alive without a GUI.
fn run() -> Result<()> {
    let io = Io::new().context("cannot start the service runtime")?;
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("--status") => return io.handle().block_on(status(false, None)),
        Some("--wait") => return io.handle().block_on(status(true, None)),
        Some("--wait-for") => {
            let pid = arguments
                .next()
                .context("a server process ID is required")?
                .parse()
                .context("cannot parse the server process ID")?;
            return io.handle().block_on(status(true, Some(pid)));
        }
        Some(argument) => bail!("unknown server argument: {argument}"),
        None => {}
    }
    let listener = {
        let handle = io.handle();
        let _entered = handle.enter();
        protocol::listen().context("cannot bind the service socket")?
    };
    let runtime = io.clone();
    service_runtime::App::run(runtime.handle(), move |cx| {
        service::start(io, cx);
        service::server::attach(listener, cx);
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        service::attach_remote(None, cx);
        let session = Sonora::global(cx).session.clone();
        session.update(cx, |session, cx| session.restore(cx));
        service::server::shutdown_on_signal(cx);
        log::info!("server: ready");
    });
    Ok(())
}

/// Probes the actual protocol; readiness is never inferred from a socket file's existence.
async fn status(wait: bool, expected: Option<u32>) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        match protocol::Client::connect().await {
            Ok(client) => {
                anyhow::ensure!(
                    expected.is_none_or(|expected| client.pid() == expected),
                    "the endpoint belongs to another server process"
                );
                let state = client.snapshot().await?;
                println!("{}", serde_json::to_string(&state)?);
                return Ok(());
            }
            Err(error) if !wait || tokio::time::Instant::now() >= deadline => return Err(error),
            Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }
}
