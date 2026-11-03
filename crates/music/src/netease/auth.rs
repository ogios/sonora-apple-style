//! NetEase QR authorization and persisted `MUSIC_U` sessions, with their optional CSRF token.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{PromptSink, SignInFailure, SignInProblem, SignInPrompt, credentials};

use super::client;

/// The cookie that proves a signed-in NetEase session.
pub(crate) const PROOF: &[&str] = &["MUSIC_U"];

const QR_POLL: Duration = Duration::from_secs(2);
const QR_LIFETIME: Duration = Duration::from_secs(180);

/// The reusable session proof, kept on the server and never included in presentation snapshots.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Credentials {
    /// The `MUSIC_U` cookie value alone, not the whole header.
    pub music_u: String,
    /// The `__csrf` cookie, which writes such as a like carry as `csrf_token`. Reads go through
    /// without it, so a pasted `MUSIC_U` alone still signs in.
    #[serde(default)]
    pub csrf: Option<String>,
}

/// Requests one QR key and polls it until approval, expiry or the caller drops the future.
pub(crate) async fn qr(prompt: &PromptSink) -> Result<Credentials> {
    let http = client::http()?;
    let (key, _) = client::login_request(
        &http,
        "/login/qrcode/unikey",
        json!({"type": 1, "noCheckToken": true, "csrf_token": ""}),
    )
    .await?;
    ensure!(
        key["code"] == 200,
        "netease refused to create a QR login key"
    );
    let key = key["unikey"]
        .as_str()
        .filter(|key| !key.is_empty() && key.len() <= 256)
        .context("the netease QR login key is missing or invalid")?;
    let url = format!(
        "https://music.163.com/login?codekey={}",
        crate::escape::component(key)
    );
    prompt(SignInPrompt::QrCode {
        url: url.clone(),
        scanned: false,
    });
    let deadline = tokio::time::Instant::now() + QR_LIFETIME;
    let mut scanned = false;
    loop {
        let poll = async {
            tokio::time::sleep(QR_POLL).await;
            client::login_request(
                &http,
                "/login/qrcode/client/login",
                json!({"type": 1, "key": key, "noCheckToken": true, "csrf_token": ""}),
            )
            .await
        };
        let (answer, headers) = tokio::time::timeout_at(deadline, poll)
            .await
            .map_err(|_| SignInFailure(SignInProblem::Cancelled))
            .context("the netease QR login expired")??;
        match answer["code"].as_i64() {
            Some(801) => {}
            Some(802) if !scanned => {
                scanned = true;
                prompt(SignInPrompt::QrCode {
                    url: url.clone(),
                    scanned,
                });
            }
            Some(802) => {}
            Some(803) => {
                let cookies = headers
                    .get_all(reqwest::header::SET_COOKIE)
                    .iter()
                    .filter_map(|cookie| cookie.to_str().ok())
                    .filter_map(|cookie| cookie.split(';').next())
                    .collect::<Vec<_>>()
                    .join("; ");
                return parse(&cookies)
                    .context("the approved netease QR login has no valid session cookie");
            }
            Some(800) => {
                return Err(SignInFailure(SignInProblem::Cancelled))
                    .context("the netease QR login expired or was cancelled");
            }
            Some(code) => bail!("netease refused the QR login with code {code}"),
            None => bail!("the netease QR login status is missing"),
        }
    }
}

fn path() -> PathBuf {
    credentials::dir("netease").join(credentials::FILE)
}

/// Reads the session out of what the sign-in window or the user hands over: a full `Cookie`
/// header, a lone `MUSIC_U=…` pair, or the bare token. Refuses anything without a `MUSIC_U`.
pub(crate) fn parse(input: &str) -> Result<Credentials> {
    let trimmed = input.trim();
    let mut music_u = None;
    let mut csrf = None;
    for pair in trimmed.split(';') {
        let Some((name, value)) = pair.trim().split_once('=') else {
            continue;
        };
        let value = value.trim();
        match name.trim() {
            "MUSIC_U" if valid(value) => music_u = Some(value.to_owned()),
            "MUSIC_U" => bail!("the MUSIC_U cookie does not look like one"),
            "__csrf" if !value.is_empty() => csrf = Some(value.to_owned()),
            _ => {}
        }
    }
    if music_u.is_none() && valid(trimmed) {
        music_u = Some(trimmed.to_owned());
    }
    let Some(music_u) = music_u else {
        bail!("the cookies carry no MUSIC_U; sign in to music.163.com first");
    };
    Ok(Credentials { music_u, csrf })
}

/// A `MUSIC_U` is a long run of letters and digits, a few hundred characters in practice.
fn valid(value: &str) -> bool {
    value.len() >= 32 && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

pub(crate) fn load() -> Option<Credentials> {
    let bytes = std::fs::read(path()).ok()?;
    let credentials: Credentials = serde_json::from_slice(&bytes).ok()?;
    valid(&credentials.music_u).then_some(credentials)
}

pub(crate) fn store(credentials: &Credentials) -> Result<()> {
    let bytes =
        serde_json::to_vec_pretty(credentials).context("cannot serialize netease credentials")?;
    credentials::write(&path(), &bytes).context("cannot store netease credentials")
}

pub(crate) fn forget() {
    credentials::remove(&path());
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "00A1B2C3D4E5F60718293A4B5C6D7E8F90A1B2C3D4E5F6071829";

    #[test]
    fn reads_a_whole_cookie_header() {
        let header = format!("NMTID=abc; MUSIC_U={TOKEN}; __csrf=deadbeef; os=pc");
        let credentials = parse(&header).expect("the header carries a session");
        assert_eq!(credentials.music_u, TOKEN);
        assert_eq!(credentials.csrf.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn reads_a_bare_token() {
        let credentials = parse(&format!("  {TOKEN}\n")).expect("a bare token is a session");
        assert_eq!(credentials.music_u, TOKEN);
        assert_eq!(credentials.csrf, None);
    }

    #[test]
    fn refuses_cookies_without_a_session() {
        assert!(parse("NMTID=abc; __csrf=deadbeef").is_err());
        assert!(parse("MUSIC_U=short").is_err());
        assert!(parse("").is_err());
    }
}
