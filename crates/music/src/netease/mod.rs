//! NetEase Cloud Music: sign-in with the browser's `MUSIC_U` cookie, the account's library,
//! catalogue and plain MP3 or FLAC streams over the endpoints its web player uses, and synced
//! lyrics from its public endpoints.
//!
//! The protocol work is ported from Mineral's NetEase channel (MIT, © 2025 wanger,
//! <https://github.com/10knamesmore/Mineral>). Only what the signed-in account may hear plays:
//! a song behind a plan the account lacks, or one NetEase only previews, is unavailable.

mod auth;
mod client;
mod crypto;
mod links;
mod lyrics;
mod playback;
mod wire;

use std::sync::Arc;

use anyhow::{Context as _, Result};
use async_trait::async_trait;

use crate::netease::playback::Factory;
use crate::{
    Capabilities, InputSource, MusicProvider, PromptSink, ProviderSession, Shape, SignIn,
    SignInPrompt, WebSignIn,
};

pub use client::NetEaseClient;
pub use lyrics::NetEase;

/// The page the sign-in window opens, the website with its sign-in dialog up.
const SIGN_IN_URL: &str = "https://music.163.com/#/login";
/// Once the cookies land, the window navigates here on its own.
const LANDING: &str = "music.163.com";
/// The cookie filter runs on bare domains (see `webview`'s `matches`), so no leading dot.
const COOKIE_DOMAIN: &str = "music.163.com";
/// What a WebKit engine says about itself; consistent with the webview's actual engine.
const AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15";

pub struct NetEaseProvider;

impl NetEaseProvider {
    pub fn new() -> Self {
        Self
    }

    async fn connect(cookies: &str) -> Result<ProviderSession> {
        let credentials = auth::parse(cookies)?;
        let client = NetEaseClient::connect(&credentials)
            .await
            .context("cannot reach netease with that session")?;
        auth::store(&credentials)?;
        Ok(session(client))
    }

    async fn restore_stored() -> Result<Option<ProviderSession>> {
        let Some(remembered) = auth::load() else {
            return Ok(None);
        };
        match NetEaseClient::connect(&remembered).await {
            Ok(client) => Ok(Some(session(client))),
            Err(error) if crate::trouble::offline(&format!("{error:#}")) => Err(error),
            Err(error) => {
                log::warn!("netease: the stored session is no longer usable: {error:#}");
                Ok(None)
            }
        }
    }
}

fn session(client: NetEaseClient) -> ProviderSession {
    ProviderSession {
        profile: client.account().clone(),
        api: Arc::new(client.clone()),
        playback: Arc::new(Factory::new(client)),
        shape: Shape::Saved,
        authenticated: true,
        capabilities: Capabilities {
            playlist_ownership: true,
            playcounts: false,
            // Songs are streamed for every play and never kept or downloaded.
            keeps: false,
            downloads: false,
            qualities: client::QUALITIES,
            ..Capabilities::ALL
        },
    }
}

impl Default for NetEaseProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MusicProvider for NetEaseProvider {
    fn name(&self) -> &'static str {
        "NetEase Cloud Music"
    }

    fn slug(&self) -> &'static str {
        "netease"
    }

    fn link(&self, link: &str) -> Option<crate::link::Linked> {
        links::read(link)
    }

    fn reach(&self) -> Option<String> {
        Some("music.163.com".to_owned())
    }

    fn sign_in_options(&self) -> Vec<SignIn> {
        vec![SignIn::Secret]
    }

    fn stored(&self) -> bool {
        auth::load().is_some()
    }

    fn public_art(&self) -> bool {
        // covers live on p*.music.126.net, open to anyone with the url
        true
    }

    async fn restore(&self) -> Result<Option<ProviderSession>> {
        Self::restore_stored().await
    }

    async fn sign_in(
        &self,
        method: SignIn,
        prompt: PromptSink,
        mut input: InputSource,
    ) -> Result<ProviderSession> {
        match method {
            SignIn::Secret | SignIn::Default => {
                prompt(SignInPrompt::Secret);
                let cookies = input.recv().await.context("sign-in was cancelled")?;
                Self::connect(&cookies).await
            }
            SignIn::Anonymous => Err(anyhow::anyhow!("netease has no anonymous sign-in")),
            SignIn::Path(_) => Err(anyhow::anyhow!(
                "netease does not sign in with a folder path"
            )),
            SignIn::Credentials { .. } => Err(anyhow::anyhow!(
                "netease does not sign in with a server address"
            )),
        }
    }

    fn sign_out(&self) {
        auth::forget();
    }

    fn web_sign_in(&self) -> Option<WebSignIn> {
        Some(WebSignIn {
            url: SIGN_IN_URL,
            landing: LANDING,
            domain: COOKIE_DOMAIN,
            proof: auth::PROOF,
            // As with Deezer, the Safari string matches the WebKit engine the Linux backend
            // drives, so an anti-bot check comparing the two finds nothing amiss.
            agent: Some(AGENT),
        })
    }
}
