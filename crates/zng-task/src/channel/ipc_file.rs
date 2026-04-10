use std::{mem, sync::atomic::AtomicUsize};

use serde::{Deserialize, Serialize};

/// File handle that can be transferred to another process.
///
/// # File
///
/// This type can be converted  from and to [`std::fs::File`]. This type does not
/// implement IO traits, it must be converted to read/write. The file handle is only closed on drop
/// if it was not sent or converted.
///
/// # Serialization
///
/// This type implements serialization only for compatibility with IPC channel, attempting to
/// serialize it outside of [`with_ipc_serialization`] context will return an error.
///
/// [`with_ipc_serialization`]: crate::channel::with_ipc_serialization
pub struct IpcFileHandle {
    handle: AtomicUsize,
}
impl From<std::fs::File> for IpcFileHandle {
    fn from(file: std::fs::File) -> Self {
        #[cfg(not(any(windows, unix)))]
        compile_error!("ipc_file mod should not be compiled in this os");

        #[cfg(windows)]
        let handle = std::os::windows::io::IntoRawHandle::into_raw_handle(file) as usize;
        #[cfg(unix)]
        let handle = std::os::fd::IntoRawFd::into_raw_fd(file) as usize;
        Self {
            handle: AtomicUsize::new(handle),
        }
    }
}
impl From<IpcFileHandle> for std::fs::File {
    fn from(mut f: IpcFileHandle) -> Self {
        let handle = mem::take(f.handle.get_mut());
        assert!(handle != 0, "cannot access file, already moved");
        // SAFETY: handle was not moved (not zero) and was converted from File
        unsafe { into_file(handle) }
    }
}
impl From<IpcFileHandle> for crate::fs::File {
    fn from(f: IpcFileHandle) -> Self {
        crate::fs::File::from(std::fs::File::from(f))
    }
}
impl Drop for IpcFileHandle {
    fn drop(&mut self) {
        let handle = mem::take(self.handle.get_mut());
        if handle != 0 {
            // SAFETY: handle was not moved (not zero) and was converted from File
            drop(unsafe { into_file(handle) });
        }
    }
}
unsafe fn into_file(handle: usize) -> std::fs::File {
    #[cfg(windows)]
    unsafe {
        std::os::windows::io::FromRawHandle::from_raw_handle(handle as _)
    }
    #[cfg(unix)]
    unsafe {
        std::os::fd::FromRawFd::from_raw_fd(handle as _)
    }
}
impl Serialize for IpcFileHandle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if !crate::channel::is_ipc_serialization() {
            return Err(serde::ser::Error::custom("cannot serialize `IpcFileHandle` outside IPC"));
        }
        let handle = self.handle.swap(0, std::sync::atomic::Ordering::Relaxed);
        assert!(handle != 0, "IpcFile already moved");
        // SAFETY: handle was not moved (not zero) and was converted from File
        let handle = unsafe { into_file(handle) };

        #[cfg(windows)]
        {
            // -> Sends a channel sender to receive the target process id and a sender to continue the protocol
            // <- Receives the target process id, DuplicateHandle
            // -> Sends the new handle and a confirmation sender
            // <- Receives confirmation, drops this handle

            // ->
            let (s, mut r) =
                super::ipc_unbounded::<(u32, super::IpcSender<(usize, super::IpcSender<bool>)>)>().map_err(serde::ser::Error::custom)?;
            let ok = Serialize::serialize(&s, serializer)?;

            // <-
            blocking::unblock(move || {
                let _hold = &handle;
                if let Ok((process_id, mut shared_sender)) = r.recv_blocking() {
                    use std::os::windows::io::AsRawHandle as _;
                    if let Some(handle) = duplicate_handle(process_id, handle.as_raw_handle() as usize) {
                        // ->
                        if let Ok((s, mut r)) = super::ipc_unbounded()
                            && shared_sender.send_blocking((handle, s)).is_ok()
                        {
                            // <-
                            let _ = r.recv_blocking();
                        }
                    }
                    // else sender will disconnect
                }
            })
            .detach();
            Ok(ok)
        }
        #[cfg(unix)]
        {
            // -> Sends a channel sender to get a socket name from target process and a sender to continue the protocol
            // <- Receives socket name and and connects UnixDatagram
            // ~> Sends the FD using datagram
            // <- Receives confirmation, drops this handle

            // ->
            let (s, mut r) = super::ipc_unbounded::<(String, super::IpcReceiver<bool>)>().map_err(serde::ser::Error::custom)?;
            let ok = Serialize::serialize(&s, serializer)?;

            // <-
            blocking::unblock(move || {
                let _hold = &handle;

                match r.recv_blocking() {
                    Ok((socket, mut confirm_rcv)) => match std::os::unix::net::UnixDatagram::unbound() {
                        Ok(datagram) => {
                            #[cfg(target_os = "linux")]
                            let result = if let Some(socket) = socket.strip_prefix('\0') {
                                use std::os::{linux::net::SocketAddrExt as _, unix::net::SocketAddr};
                                datagram.connect_addr(&SocketAddr::from_abstract_name(socket.as_bytes()).unwrap())
                            } else {
                                let socket = std::path::PathBuf::from("/tmp/").join(socket);
                                datagram.connect(&socket)
                            };
                            #[cfg(not(target_os = "linux"))]
                            let result = {
                                let socket = std::path::PathBuf::from("/tmp/").join(socket);
                                datagram.connect(&socket)
                            };
                            match result {
                                Ok(()) => {
                                    // ~>
                                    use sendfd::SendWithFd as _;
                                    use std::os::fd::AsRawFd as _;
                                    match datagram.send_with_fd(b"zng", &[handle.as_raw_fd()]) {
                                        Ok(_) => {
                                            // <-
                                            let _ = confirm_rcv.recv_blocking();
                                        }
                                        Err(e) => tracing::error!("cannot send IpcFileHandle, {e}"),
                                    }
                                }
                                Err(e) => tracing::error!("cannot send IpcFileHandle, cannot connect socket, {e}"),
                            }
                        }
                        Err(e) => tracing::error!("cannot send IpcFileHandle, cannot create unbound datagram, {e}"),
                    },
                    Err(e) => tracing::error!("cannot send IpcFileHandle, side channel disconnected, {e}"),
                }
            })
            .detach();

            Ok(ok)
        }

        #[cfg(not(any(windows, unix)))]
        {
            panic!("IpcFileHandle not implemented for {}", std::env::consts::OS);
        }
    }
}
impl<'de> Deserialize<'de> for IpcFileHandle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[cfg(windows)]
        {
            type Confirm = bool;
            type Handle = (usize, super::IpcSender<Confirm>);
            type Process = (u32, super::IpcSender<Handle>);

            let mut process_id_sender = <super::IpcSender<Process> as Deserialize<'de>>::deserialize(deserializer)?;
            let (s, mut handle_receiver) = super::ipc_unbounded::<Handle>().map_err(serde::de::Error::custom)?;

            process_id_sender
                .send_blocking((std::process::id(), s))
                .map_err(serde::de::Error::custom)?;

            let (handle, mut confirm_sender) = handle_receiver.recv_blocking().map_err(serde::de::Error::custom)?;

            use std::os::windows::io::FromRawHandle as _;
            // SAFETY: this handle is the output of DuplicateHandle for the current process
            let handle = unsafe { std::fs::File::from_raw_handle(handle as _) };

            let _ = confirm_sender.send_blocking(true);

            Ok(handle.into())
        }

        #[cfg(unix)]
        {
            let mut socket_sender = <super::IpcSender<(String, super::IpcReceiver<bool>)> as Deserialize<'de>>::deserialize(deserializer)?;

            static SOCKET_ID: AtomicUsize = AtomicUsize::new(0);
            let mut socket = format!(
                "zng_task-ipc_file-{}-{}",
                std::process::id(),
                SOCKET_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            );
            let mut socket_tmp = None;

            use std::os::unix::net::UnixDatagram;
            #[cfg(target_os = "linux")]
            let fd_recv = {
                // try abstract name first
                use std::os::{linux::net::SocketAddrExt as _, unix::net::SocketAddr};
                match UnixDatagram::bind_addr(&SocketAddr::from_abstract_name(socket.as_bytes()).unwrap()) {
                    Ok(r) => {
                        socket = format!("\0{socket}");
                        r
                    }
                    Err(e) => {
                        if matches!(e.kind(), std::io::ErrorKind::InvalidInput) {
                            // fallback to tmp file socket
                            let socket = std::path::PathBuf::from("/tmp/").join(&socket);
                            let _ = std::fs::remove_file(&socket);
                            let r = UnixDatagram::bind(&socket).map_err(serde::de::Error::custom)?;
                            socket_tmp = Some(socket);
                            r
                        } else {
                            return Err(serde::de::Error::custom(e));
                        }
                    }
                }
            };
            #[cfg(not(target_os = "linux"))]
            let fd_recv = {
                let socket = std::path::PathBuf::from("/tmp/").join(&socket);
                let _ = std::fs::remove_file(&socket);
                let r = nixDatagram::bind(socket).map_err(serde::de::Error::custom)?;
                socket_tmp = Some(socket);
                r
            };
            let _cleanup = zng_app_context::RunOnDrop::new(move || {
                if let Some(socket) = socket_tmp {
                    let _ = std::fs::remove_file(socket);
                }
            });

            let (mut confirm_sender, r) = super::ipc_unbounded().map_err(serde::de::Error::custom)?;
            socket_sender.send_blocking((socket, r)).map_err(serde::de::Error::custom)?;

            use sendfd::RecvWithFd as _;
            let mut ignore = [b'z', b'n', b'g'];
            let mut fd = [0 as std::os::fd::RawFd];
            fd_recv.recv_with_fd(&mut ignore, &mut fd).map_err(serde::de::Error::custom)?;

            use std::os::fd::FromRawFd as _;
            let handle = unsafe { std::fs::File::from_raw_fd(fd[0]) };
            let _ = confirm_sender.send_blocking(true);

            Ok(handle.into())
        }

        #[cfg(not(any(windows, unix)))]
        {
            panic!("IpcFile not implemented for {}", std::env::consts::OS);
        }
    }
}

#[cfg(windows)]
fn duplicate_handle(process_id: u32, handle: usize) -> Option<usize> {
    unsafe {
        use windows_sys::Win32::Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE};
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_DUP_HANDLE};

        let target_process = OpenProcess(PROCESS_DUP_HANDLE, 0, process_id);
        if target_process != 0 {
            let mut target_handle: HANDLE = 0;
            let success = DuplicateHandle(
                GetCurrentProcess(),
                handle as HANDLE,
                target_process,
                &mut target_handle,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            );

            windows_sys::Win32::Foundation::CloseHandle(target_process);

            if success != 0 {
                Some(target_handle as usize)
            } else {
                tracing::error!("failed to duplicate IpcFile handle");
                None
            }
        } else {
            tracing::error!("failed to connect to target process for IpcFile handle duplication");
            None
        }
    }
}
