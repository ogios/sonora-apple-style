use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, bail, ensure};
use i18n::t;
use protocol::Call;
use serde_json::json;

/// A validated terminal operation; parsing completes before connecting to the service.
pub enum Command {
    Help,
    Version,
    Status,
    Watch,
    Query(&'static str),
    Call(Call),
    Track {
        id: String,
        enqueue: bool,
    },
    Search(String),
    Queue {
        method: &'static str,
        index: Option<usize>,
    },
    Login {
        provider: String,
        method: String,
    },
    Input,
}

/// Output preferences and the single operation requested by this invocation.
pub struct Arguments {
    pub json: bool,
    pub command: Command,
}

impl Arguments {
    pub fn parse() -> Result<Self> {
        let mut arguments = Vec::new();
        let mut json = false;
        let mut options = true;
        for argument in std::env::args().skip(1) {
            match argument.as_str() {
                "--" if options => options = false,
                "--json" if options => json = true,
                "--help" | "-h" if options => {
                    return Ok(Self {
                        json,
                        command: Command::Help,
                    });
                }
                "--version" | "-V" if options => {
                    return Ok(Self {
                        json,
                        command: Command::Version,
                    });
                }
                _ => arguments.push(argument),
            }
        }
        let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let command = match words.as_slice() {
            [] | ["status"] => Command::Status,
            ["help"] => Command::Help,
            ["watch"] => Command::Watch,
            ["play"] => call("playback", "resume", vec![]),
            ["play", id] => Command::Track {
                id: (*id).to_owned(),
                enqueue: false,
            },
            ["enqueue", id] => Command::Track {
                id: (*id).to_owned(),
                enqueue: true,
            },
            ["pause"] => call("playback", "pause", vec![]),
            ["toggle"] => call("playback", "toggle_play", vec![]),
            ["next"] => call("playback", "next", vec![]),
            ["previous"] => call("playback", "previous", vec![]),
            ["seek", seconds] => {
                let seconds = number(seconds)?;
                let duration = Duration::try_from_secs_f64(seconds)
                    .map_err(|_| anyhow::anyhow!("{}", t!("cli-invalid-number")))?;
                call("playback", "seek", vec![json!(duration)])
            }
            ["volume"] | ["repeat"] | ["shuffle"] => Command::Query("playback"),
            ["volume", percent] => {
                let percent = number(percent)?;
                ensure!(percent <= 100., "{}", t!("cli-invalid-volume"));
                call("playback", "set_volume", vec![json!(percent / 100.)])
            }
            ["repeat", mode @ ("off" | "all" | "one")] => {
                call("playback", "set_repeat", vec![json!(mode)])
            }
            ["shuffle", "toggle"] => call("playback", "toggle_shuffle", vec![]),
            ["shuffle", mode @ ("on" | "off")] => {
                call("playback", "set_shuffle", vec![json!(*mode == "on")])
            }
            ["queue"] | ["queue", "list"] => Command::Query("queue"),
            ["queue", "clear"] => Command::Queue {
                method: "clear_upcoming",
                index: None,
            },
            ["queue", action @ ("play" | "remove"), index] => {
                let index: usize = index
                    .parse()
                    .ok()
                    .filter(|index| *index > 0)
                    .ok_or_else(|| anyhow::anyhow!("{}", t!("cli-invalid-index")))?;
                let method = match *action {
                    "play" => "play_upcoming",
                    _ => "remove_upcoming",
                };
                Command::Queue {
                    method,
                    index: Some(index - 1),
                }
            }
            ["open", paths @ ..] if !paths.is_empty() => {
                let paths = paths
                    .iter()
                    .map(|path| absolute_file(path))
                    .collect::<Result<Vec<_>>>()?;
                call("playback", "open_paths", vec![json!(paths)])
            }
            ["search", query @ ..] if !query.is_empty() => {
                let query = query.join(" ");
                ensure!(!query.trim().is_empty(), "{}", t!("cli-usage-error"));
                Command::Search(query)
            }
            ["account"] | ["account", "status"] => Command::Query("session"),
            ["account", "switch", provider] => call("session", "switch", vec![json!(provider)]),
            ["account", "sign-out"] => call("session", "sign_out", vec![]),
            ["account", "cancel"] => call("session", "cancel_sign_in", vec![]),
            ["account", "login", provider] => Command::Login {
                provider: (*provider).to_owned(),
                method: "Default".to_owned(),
            },
            [
                "account",
                "login",
                provider,
                method @ ("default" | "qr" | "anonymous" | "secret"),
            ] => {
                let method = match *method {
                    "qr" => "QrCode",
                    "anonymous" => "Anonymous",
                    "secret" => "Secret",
                    _ => "Default",
                };
                Command::Login {
                    provider: (*provider).to_owned(),
                    method: method.to_owned(),
                }
            }
            ["account", "input"] => Command::Input,
            ["local", "add", path] => {
                let path = std::fs::canonicalize(path)
                    .map_err(|error| anyhow::anyhow!("{}: {error}", t!("cli-invalid-path")))?;
                ensure!(path.is_dir(), "{}", t!("cli-invalid-path"));
                call("session", "add_local_folder", vec![json!(path)])
            }
            ["local", "rescan"] => call("session", "rescan_local", vec![json!(true)]),
            _ => bail!("{}", t!("cli-usage-error")),
        };
        Ok(Self { json, command })
    }
}

fn call(object: &str, method: &str, arguments: Vec<serde_json::Value>) -> Command {
    Command::Call(Call::new(object, method, arguments))
}

/// Accepts finite, nonnegative numbers so invalid values never reach IPC serialization.
fn number(text: &str) -> Result<f64> {
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.)
        .ok_or_else(|| anyhow::anyhow!("{}", t!("cli-invalid-number")))
}

/// Resolves paths in the caller's working directory before the server sees them.
fn absolute_file(path: &str) -> Result<PathBuf> {
    let path = std::fs::canonicalize(path)
        .map_err(|error| anyhow::anyhow!("{}: {error}", t!("cli-invalid-path")))?;
    ensure!(path.is_file(), "{}", t!("cli-invalid-path"));
    Ok(path)
}
