//! Vault credentials in Windows Credential Manager, using the existing target identity.

use anyhow::{Result, ensure};
use windows::Win32::Foundation::ERROR_NOT_FOUND;
use windows::Win32::Security::Credentials::{
    CRED_MAX_CREDENTIAL_BLOB_SIZE, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW,
    CredFree, CredReadW, CredWriteW,
};
use windows::core::{PCWSTR, PWSTR};

/// Reads a credential and releases the native allocation after copying its values.
pub fn read(url: &str) -> Result<Option<(String, Vec<u8>)>> {
    let target = target(url);
    let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: Target is NUL terminated and the out-pointer is valid.
    if let Err(error) = unsafe {
        CredReadW(
            PCWSTR(target.as_ptr()),
            CRED_TYPE_GENERIC,
            None,
            &mut credential,
        )
    } {
        if error.code() == ERROR_NOT_FOUND.to_hresult() {
            return Ok(None);
        }
        return Err(error.into());
    }
    if credential.is_null() {
        return Ok(None);
    }
    // SAFETY: CredReadW returned an allocated credential valid until CredFree.
    let result = unsafe { copy(&*credential) };
    // SAFETY: This allocation was returned by CredReadW and is released exactly once.
    unsafe { CredFree(credential.cast()) };
    result.map(Some)
}

/// Writes the vault master key without creating a window or a platform event loop.
pub fn write(url: &str, account: &str, password: &[u8]) -> Result<()> {
    ensure!(
        password.len() <= CRED_MAX_CREDENTIAL_BLOB_SIZE as usize,
        "the credential exceeds the Windows blob size limit"
    );
    let mut target = target(url);
    let mut account = account.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target.as_mut_ptr()),
        CredentialBlobSize: password.len() as u32,
        CredentialBlob: password.as_ptr().cast_mut(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: PWSTR(account.as_mut_ptr()),
        ..CREDENTIALW::default()
    };
    // SAFETY: All credential buffers remain live for the duration of CredWriteW.
    unsafe { CredWriteW(&credential, 0)? };
    Ok(())
}

fn target(url: &str) -> Vec<u16> {
    format!("zed:url={url}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

/// Copies fields from a live native credential without taking ownership of its buffers.
unsafe fn copy(credential: &CREDENTIALW) -> Result<(String, Vec<u8>)> {
    let account = match credential.UserName.is_null() {
        true => String::new(),
        // SAFETY: The caller supplies a live CREDENTIALW with a terminated account name.
        false => unsafe { credential.UserName.to_string()? },
    };
    let password = match credential.CredentialBlob.is_null() {
        true => Vec::new(),
        // SAFETY: The caller supplies CredentialBlobSize bytes in this live credential.
        false => unsafe {
            std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            )
        }
        .to_vec(),
    };
    Ok((account, password))
}
