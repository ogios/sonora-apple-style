use std::io::{self, Write as _};

use anyhow::{Context as _, Result};
use i18n::t;
use qrcodegen::{QrCode, QrCodeEcc};
use serde_json::{Value, json};

const QR_BORDER: i32 = 4;

/// Writes through fallible stdout so a closed pipeline terminates without a panic.
pub fn line(text: &str) -> Result<()> {
    writeln!(io::stdout().lock(), "{text}").context("cannot write CLI output")
}

/// A command acknowledgement means the server accepted it; provider work may still be pending.
pub fn accepted(json_output: bool) -> Result<()> {
    match json_output {
        true => line(&json!({"accepted": true}).to_string()),
        false => line(&t!("cli-accepted")),
    }
}

/// Presents one server read model without loading GUI types into the terminal client.
pub fn show(name: &str, value: &Value, json_output: bool) -> Result<()> {
    if json_output {
        return line(&serde_json::to_string(value)?);
    }
    match name {
        "playback" => playback(value),
        "queue" => queue(value),
        "search" => search(value),
        "session" => session(value),
        _ => unreachable!(),
    }
}

fn playback(value: &Value) -> Result<()> {
    let state = match value["state"].as_str() {
        Some("Playing") => t!("cli-state-playing").to_string(),
        Some("Paused") => t!("cli-state-paused").to_string(),
        Some("Loading") => t!("cli-state-loading").to_string(),
        Some("Idle") => t!("cli-state-idle").to_string(),
        _ => t!("cli-state-failed", reason = text(&value["state"]["Failed"])).to_string(),
    };
    line(&state)?;
    let track = &value["track"];
    if !track.is_null() {
        line(&track_row(track))?;
    }
    line(&t!(
        "cli-playback-details",
        position = clock(&value["position"]),
        duration = clock(&value["duration"]),
        volume = format!("{:.0}", value["volume"].as_f64().unwrap_or(0.) * 100.),
        repeat = match value["repeat"].as_str() {
            Some("all") => t!("cli-repeat-all").to_string(),
            Some("one") => t!("cli-repeat-one").to_string(),
            _ => t!("cli-off").to_string(),
        },
        shuffle = match value["shuffle"] == true {
            true => t!("cli-on").to_string(),
            false => t!("cli-off").to_string(),
        },
    ))
}

fn queue(value: &Value) -> Result<()> {
    line(&t!("cli-queue-current"))?;
    match value["current"].get("track") {
        Some(track) => line(&track_row(track))?,
        None => line(&t!("cli-empty"))?,
    }
    line(&t!("cli-queue-upcoming"))?;
    let entries = value["upcoming"]
        .as_array()
        .context("the queue is missing upcoming tracks")?;
    if entries.is_empty() {
        return line(&t!("cli-empty"));
    }
    for (index, entry) in entries.iter().enumerate() {
        line(&format!("{}\t{}", index + 1, track_row(&entry["track"])))?;
    }
    Ok(())
}

fn search(value: &Value) -> Result<()> {
    let hits = value["hits"]
        .as_array()
        .context("the search model is missing results")?;
    if hits.is_empty() {
        return line(&t!("cli-empty"));
    }
    for hit in hits {
        if let Some(track) = hit.get("Song") {
            line(&format!("{}\t{}", t!("cli-kind-song"), track_row(track)))?;
        } else if let Some(album) = hit.get("Album") {
            line(&format!(
                "{}\t{}\t{}\t{}",
                t!("cli-kind-album"),
                text(&album["id"]),
                text(&album["name"]),
                text(&album["artists"])
            ))?;
        } else if let Some(artist) = hit.get("Artist") {
            line(&format!(
                "{}\t{}\t{}",
                t!("cli-kind-artist"),
                text(&artist["id"]),
                text(&artist["name"])
            ))?;
        } else if let Some(playlist) = hit.get("Playlist") {
            line(&format!(
                "{}\t{}\t{}",
                t!("cli-kind-playlist"),
                text(&playlist["id"]),
                text(&playlist["name"])
            ))?;
        }
    }
    Ok(())
}

fn session(value: &Value) -> Result<()> {
    let state = match value["state"].as_str() {
        Some("SignedOut") => t!("cli-account-unsigned").to_string(),
        Some("Restoring") => t!("login-restoring").to_string(),
        _ if value["state"].get("Authorizing").is_some() => {
            t!("cli-account-authorizing").to_string()
        }
        _ if value["state"].get("SignedIn").is_some() => t!(
            "login-signed-in",
            name = text(&value["state"]["SignedIn"]["display_name"])
        )
        .to_string(),
        _ if value["state"].get("Offline").is_some() => t!("cli-account-offline").to_string(),
        _ => text(&value["state"]["Failed"]["summary"]),
    };
    line(&t!(
        "cli-account-state",
        provider = text(&value["provider_name"]),
        state = state
    ))?;
    if let Some(providers) = value["providers"].as_array() {
        for provider in providers {
            line(&format!(
                "{}\t{}\t{}",
                text(&provider["slug"]),
                text(&provider["name"]),
                match (provider["active"] == true, provider["stored"] == true) {
                    (true, _) => t!("cli-account-active"),
                    (_, true) => t!("cli-account-stored"),
                    _ => t!("cli-account-unsigned"),
                }
            ))?;
        }
    }
    if let Some(prompt) = value["state"].get("Authorizing") {
        if let Some(code) = prompt.get("Code") {
            line(&t!(
                "cli-auth-code",
                url = text(&code["url"]),
                code = text(&code["code"])
            ))?;
            if let Some(url) = code["qr_url"].as_str() {
                line(&t!("cli-auth-qr"))?;
                qr(url)?;
            }
        } else if let Some(url) = prompt.get("Url") {
            line(&t!("cli-auth-url", url = text(url)))?;
        } else if prompt == "Secret" {
            line(&t!("cli-auth-secret"))?;
        } else if let Some(accounts) = prompt.get("Accounts") {
            line(&serde_json::to_string_pretty(accounts)?)?;
            line(&t!("cli-auth-choice"))?;
        }
    }
    Ok(())
}

fn track_row(track: &Value) -> String {
    format!(
        "{}\t{}\t{}\t{}",
        text(&track["id"]),
        text(&track["name"]),
        text(&track["artists"]),
        clock(&track["duration"])
    )
}

fn clock(duration: &Value) -> String {
    let seconds = duration["secs"].as_u64().unwrap_or(0);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn text(value: &Value) -> String {
    clean(value.as_str().unwrap_or(""))
}

/// Metadata remains one terminal cell and cannot introduce control sequences.
fn clean(text: &str) -> String {
    text.chars()
        .map(|character| match character.is_control() {
            true => ' ',
            false => character,
        })
        .collect()
}

/// Uses two vertical modules per character and a four-module quiet zone for terminal scanning.
/// Light modules use the terminal foreground, so the common dark TTY background stays dark.
fn qr(url: &str) -> Result<()> {
    let code = QrCode::encode_text(url, QrCodeEcc::Medium)
        .map_err(|error| anyhow::anyhow!("cannot encode the provider's QR login: {error}"))?;
    let end = code.size() + QR_BORDER;
    for y in (-QR_BORDER..end).step_by(2) {
        let mut row = String::new();
        for x in -QR_BORDER..end {
            let light = |y| {
                x < 0 || y < 0 || x >= code.size() || y >= code.size() || !code.get_module(x, y)
            };
            row.push(match (light(y), light(y + 1)) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        line(&row)?;
    }
    Ok(())
}
