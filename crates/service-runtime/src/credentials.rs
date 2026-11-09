//! Platform credential storage, preserving the existing vault keychain identity.

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use anyhow::Context as _;
use anyhow::Result;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
const LABEL: &str = "zed-github-account";

/// Reads the account and secret without opening a windowing platform.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub async fn read(url: String) -> Result<Option<(String, Vec<u8>)>> {
    let keyring = oo7::Keyring::new().await?;
    keyring.unlock().await?;
    for item in keyring.search_items(&[("url", &url)]).await? {
        if item.label().await.is_ok_and(|label| label == LABEL) {
            let attributes = item.attributes().await?;
            let account = attributes
                .get("username")
                .context("cannot find the credential account")?
                .to_owned();
            item.unlock().await?;
            return Ok(Some((account, item.secret().await?.to_vec())));
        }
    }
    Ok(None)
}

/// Updates the same keychain item used by existing downloaded tracks.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub async fn write(url: String, account: String, password: Vec<u8>) -> Result<()> {
    let keyring = oo7::Keyring::new().await?;
    keyring.unlock().await?;
    keyring
        .create_item(
            LABEL,
            &[("url", &url), ("username", &account)],
            password,
            true,
        )
        .await?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[path = "credentials/macos.rs"]
mod macos;

#[cfg(target_os = "macos")]
pub async fn read(url: String) -> Result<Option<(String, Vec<u8>)>> {
    tokio::task::spawn_blocking(move || macos::read_credentials(&url).map_err(Into::into)).await?
}

#[cfg(target_os = "macos")]
pub async fn write(url: String, account: String, password: Vec<u8>) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        macos::write_credentials(&url, &account, &password).map_err(Into::into)
    })
    .await?
}

#[cfg(target_os = "windows")]
#[path = "credentials/windows.rs"]
mod windows;

#[cfg(target_os = "windows")]
pub async fn read(url: String) -> Result<Option<(String, Vec<u8>)>> {
    tokio::task::spawn_blocking(move || windows::read(&url)).await?
}

#[cfg(target_os = "windows")]
pub async fn write(url: String, account: String, password: Vec<u8>) -> Result<()> {
    tokio::task::spawn_blocking(move || windows::write(&url, &account, &password)).await?
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "macos",
    target_os = "windows"
)))]
pub async fn read(_: String) -> Result<Option<(String, Vec<u8>)>> {
    anyhow::bail!(
        "org.freedesktop.DBus.Error.ServiceUnknown: no credential service on this platform"
    )
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "macos",
    target_os = "windows"
)))]
pub async fn write(_: String, _: String, _: Vec<u8>) -> Result<()> {
    anyhow::bail!(
        "org.freedesktop.DBus.Error.ServiceUnknown: no credential service on this platform"
    )
}
