use std::io::{self, ErrorKind};

use interprocess::local_socket::tokio::{Listener, Stream};
use interprocess::local_socket::{ListenerOptions, Name, traits::tokio::Stream as _};

/// Binds the service endpoint exclusively, without replacing another server's listener.
pub fn listen() -> io::Result<Listener> {
    let options = ListenerOptions::new().name(name()?);
    #[cfg(windows)]
    let options = {
        use interprocess::os::windows::local_socket::ListenerOptionsExt as _;
        options.security_descriptor(reachable()?)
    };
    options.create_tokio()
}

/// Opens the endpoint selected by `SONORA_SOCKET`, or the current user's default endpoint.
pub async fn connect() -> io::Result<Stream> {
    Stream::connect(name()?).await
}

#[cfg(unix)]
fn name() -> io::Result<Name<'static>> {
    use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _};
    use std::path::PathBuf;

    use interprocess::local_socket::{GenericFilePath, ToFsName as _};

    let default = match cfg!(debug_assertions) {
        true => "server-dev.sock",
        false => "server.sock",
    };
    let path = match std::env::var_os("SONORA_SOCKET") {
        Some(path) => PathBuf::from(path),
        None => dirs::runtime_dir()
            .or_else(dirs::cache_dir)
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no runtime or cache folder"))?
            .join("sonora")
            .join("ipc")
            .join(default),
    };
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidInput,
                "the service socket needs a parent folder",
            )
        })?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(parent)?;
    let metadata = std::fs::symlink_metadata(parent)?;
    // SAFETY: geteuid reads the current process's effective user ID and has no preconditions.
    let owner = unsafe { libc::geteuid() };
    if !metadata.is_dir() || metadata.uid() != owner || metadata.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            ErrorKind::PermissionDenied,
            "the service socket folder must be private and owned by this user",
        ));
    }
    path.to_fs_name::<GenericFilePath>()
}

#[cfg(windows)]
fn name() -> io::Result<Name<'static>> {
    use interprocess::local_socket::{GenericNamespaced, ToNsName as _};

    let name = match std::env::var("SONORA_SOCKET") {
        Ok(name) => name,
        Err(_) => format!("sonora-server-{}-{}", user_sid()?, cfg!(debug_assertions)),
    };
    name.to_ns_name::<GenericNamespaced>()
}

/// Grants pipe access to the current account, including its elevated processes.
#[cfg(windows)]
fn reachable() -> io::Result<interprocess::os::windows::security_descriptor::SecurityDescriptor> {
    use interprocess::os::windows::security_descriptor::SecurityDescriptor;

    let sid = user_sid()?;
    let descriptor = widestring::U16CString::from_str(format!("D:(A;;GA;;;{sid})"))
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
    SecurityDescriptor::deserialize(&descriptor)
}

/// The SID of the account this process runs as, in its `S-1-5-…` text form.
#[cfg(windows)]
fn user_sid() -> io::Result<String> {
    use std::ptr;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token: HANDLE = ptr::null_mut();
    // SAFETY: the pseudo handle of the current process needs no closing, and `token` is a live
    // out pointer.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }

    let mut length = 0u32;
    // SAFETY: a null buffer of length zero only asks for the size the answer needs.
    unsafe { GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut length) };
    // u64 cells keep the buffer aligned for the pointer at the start of TOKEN_USER
    let mut buffer = vec![0u64; (length as usize).div_ceil(size_of::<u64>())];
    // SAFETY: the buffer holds at least `length` bytes, and the token is open until closed below.
    let read = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            length,
            &mut length,
        )
    };
    let error = io::Error::last_os_error();
    // SAFETY: the token was opened above and is not used again.
    unsafe { CloseHandle(token) };
    if read == 0 {
        return Err(error);
    }

    // SAFETY: GetTokenInformation filled the buffer with a TOKEN_USER whose SID points into it.
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    let mut text = ptr::null_mut();
    // SAFETY: `sid` is valid while `buffer` lives, and `text` is a live out pointer.
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: ConvertSidToStringSidW returned a NUL-terminated string that LocalFree releases.
    let sid = unsafe { widestring::U16CStr::from_ptr_str(text) }.to_string_lossy();
    unsafe { LocalFree(text.cast()) };
    Ok(sid)
}
