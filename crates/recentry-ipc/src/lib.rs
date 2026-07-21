//! Current-user named-pipe transport for Recentry host/UI messages.
//!
//! Each connection has one request/response owner. The synchronous handle must
//! not be cloned for concurrent reads and writes; doing so can deadlock.

use serde::{Serialize, de::DeserializeOwned};

pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("IPC is unsupported on this platform")]
    Unsupported,
    #[error("message exceeds the IPC limit")]
    MessageTooLarge,
    #[error("{operation} failed with Windows error {code}")]
    Windows { operation: &'static str, code: u32 },
    #[error("protocol encoding failed: {0}")]
    Protocol(#[from] serde_json::Error),
}

#[cfg(windows)]
mod platform {
    use std::{
        cell::Cell,
        ffi::c_void,
        marker::PhantomData,
        mem::size_of,
        ptr::{null, null_mut},
        thread,
        time::{Duration, Instant},
    };

    use serde::{Serialize, de::DeserializeOwned};
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, GetLastError,
            HANDLE, INVALID_HANDLE_VALUE, LocalFree,
        },
        Security::{
            Authorization::{
                ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
            },
            PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
        },
        Storage::FileSystem::{
            CreateFileW, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
        },
        System::Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES, PIPE_WAIT, WaitNamedPipeW,
        },
    };

    use super::{IpcError, MAX_MESSAGE_BYTES};

    pub struct PipeServer {
        name: Vec<u16>,
    }

    pub struct PipeConnection {
        handle: HANDLE,
        _not_sync: PhantomData<Cell<()>>,
    }

    unsafe impl Send for PipeConnection {}

    impl Drop for PipeConnection {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

    impl SecurityDescriptor {
        fn current_owner_only() -> Result<Self, IpcError> {
            let sddl = wide("D:P(A;;GA;;;SY)(A;;GA;;;OW)");
            let mut descriptor = null_mut();
            let ok = unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    null_mut(),
                )
            };
            if ok == 0 {
                Err(last_error("security descriptor"))
            } else {
                Ok(Self(descriptor))
            }
        }

        fn attributes(&mut self) -> SECURITY_ATTRIBUTES {
            SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: self.0.cast::<c_void>(),
                bInheritHandle: 0,
            }
        }
    }

    impl Drop for SecurityDescriptor {
        fn drop(&mut self) {
            unsafe {
                LocalFree(self.0.cast());
            }
        }
    }

    impl PipeServer {
        pub fn bind(name: &str) -> Result<Self, IpcError> {
            if !name.starts_with(r"\\.\pipe\") || name.len() > 240 {
                return Err(IpcError::Windows {
                    operation: "validate pipe name",
                    code: 87,
                });
            }
            Ok(Self { name: wide(name) })
        }

        pub fn accept(&self) -> Result<PipeConnection, IpcError> {
            let mut descriptor = SecurityDescriptor::current_owner_only()?;
            let attributes = descriptor.attributes();
            let handle = unsafe {
                CreateNamedPipeW(
                    self.name.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                    PIPE_UNLIMITED_INSTANCES,
                    64 * 1024,
                    64 * 1024,
                    5_000,
                    &attributes,
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(last_error("CreateNamedPipeW"));
            }
            let connected = unsafe { ConnectNamedPipe(handle, null_mut()) };
            if connected == 0 && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
                unsafe {
                    CloseHandle(handle);
                }
                return Err(last_error("ConnectNamedPipe"));
            }
            Ok(PipeConnection {
                handle,
                _not_sync: PhantomData,
            })
        }
    }

    impl PipeConnection {
        pub fn send<T: Serialize>(&self, message: &T) -> Result<(), IpcError> {
            let payload = serde_json::to_vec(message)?;
            if payload.len() > MAX_MESSAGE_BYTES {
                return Err(IpcError::MessageTooLarge);
            }
            self.write_all(&(payload.len() as u32).to_le_bytes())?;
            self.write_all(&payload)
        }

        pub fn receive<T: DeserializeOwned>(&self) -> Result<T, IpcError> {
            let mut length = [0u8; 4];
            self.read_exact(&mut length)?;
            let length = u32::from_le_bytes(length) as usize;
            if length > MAX_MESSAGE_BYTES {
                return Err(IpcError::MessageTooLarge);
            }
            let mut payload = vec![0u8; length];
            self.read_exact(&mut payload)?;
            Ok(serde_json::from_slice(&payload)?)
        }

        fn read_exact(&self, mut buffer: &mut [u8]) -> Result<(), IpcError> {
            while !buffer.is_empty() {
                let mut read = 0u32;
                let ok = unsafe {
                    ReadFile(
                        self.handle,
                        buffer.as_mut_ptr(),
                        buffer.len().min(u32::MAX as usize) as u32,
                        &mut read,
                        null_mut(),
                    )
                };
                if ok == 0 || read == 0 {
                    return Err(last_error("ReadFile"));
                }
                buffer = &mut buffer[read as usize..];
            }
            Ok(())
        }

        fn write_all(&self, mut buffer: &[u8]) -> Result<(), IpcError> {
            while !buffer.is_empty() {
                let mut written = 0u32;
                let ok = unsafe {
                    WriteFile(
                        self.handle,
                        buffer.as_ptr(),
                        buffer.len().min(u32::MAX as usize) as u32,
                        &mut written,
                        null_mut(),
                    )
                };
                if ok == 0 || written == 0 {
                    return Err(last_error("WriteFile"));
                }
                buffer = &buffer[written as usize..];
            }
            Ok(())
        }
    }

    pub fn connect(name: &str, timeout_ms: u32) -> Result<PipeConnection, IpcError> {
        let name = wide(name);
        let started = Instant::now();
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        loop {
            let handle = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    windows_sys::Win32::Foundation::GENERIC_READ
                        | windows_sys::Win32::Foundation::GENERIC_WRITE,
                    0,
                    null(),
                    OPEN_EXISTING,
                    0,
                    null_mut(),
                )
            };
            if handle != INVALID_HANDLE_VALUE {
                return Ok(PipeConnection {
                    handle,
                    _not_sync: PhantomData,
                });
            }
            let code = unsafe { GetLastError() };
            if code != ERROR_PIPE_BUSY && code != ERROR_FILE_NOT_FOUND {
                return Err(IpcError::Windows {
                    operation: "CreateFileW",
                    code,
                });
            }
            let remaining = timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(IpcError::Windows {
                    operation: "connect timeout",
                    code,
                });
            }
            let wait_ms = remaining.as_millis().min(100) as u32;
            unsafe {
                WaitNamedPipeW(name.as_ptr(), wait_ms);
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    fn last_error(operation: &'static str) -> IpcError {
        IpcError::Windows {
            operation,
            code: unsafe { GetLastError() },
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use serde::{Serialize, de::DeserializeOwned};

    use super::IpcError;

    pub struct PipeServer;
    pub struct PipeConnection;

    impl PipeServer {
        pub fn bind(_name: &str) -> Result<Self, IpcError> {
            Err(IpcError::Unsupported)
        }

        pub fn accept(&self) -> Result<PipeConnection, IpcError> {
            Err(IpcError::Unsupported)
        }
    }

    impl PipeConnection {
        pub fn send<T: Serialize>(&self, _message: &T) -> Result<(), IpcError> {
            Err(IpcError::Unsupported)
        }

        pub fn receive<T: DeserializeOwned>(&self) -> Result<T, IpcError> {
            Err(IpcError::Unsupported)
        }
    }

    pub fn connect(_name: &str, _timeout_ms: u32) -> Result<PipeConnection, IpcError> {
        Err(IpcError::Unsupported)
    }
}

pub use platform::{PipeConnection, PipeServer, connect};

pub fn request<Req: Serialize, Resp: DeserializeOwned>(
    name: &str,
    request: &Req,
    timeout_ms: u32,
) -> Result<Resp, IpcError> {
    let connection = connect(name, timeout_ms)?;
    connection.send(request)?;
    connection.receive()
}

#[cfg(all(test, windows))]
mod tests {
    use std::{process, thread};

    use recentry_protocol::{HostCommand, HostResponse};

    use super::*;

    fn test_pipe(label: &str) -> String {
        format!(r"\\.\pipe\recentry-ipc-test-{}-{label}", process::id())
    }

    #[test]
    fn current_user_pipe_round_trips_a_protocol_message() {
        let name = test_pipe("roundtrip");
        let server = PipeServer::bind(&name).unwrap();
        let worker = thread::spawn(move || {
            let connection = server.accept().unwrap();
            assert_eq!(
                connection.receive::<HostCommand>().unwrap(),
                HostCommand::Ping
            );
            connection.send(&HostResponse::Pong).unwrap();
        });
        let response = request::<_, HostResponse>(&name, &HostCommand::Ping, 5_000).unwrap();
        assert_eq!(response, HostResponse::Pong);
        worker.join().unwrap();
    }

    #[test]
    fn transport_rejects_oversized_messages_before_writing() {
        let name = test_pipe("oversized");
        let server = PipeServer::bind(&name).unwrap();
        let worker = thread::spawn(move || {
            let connection = server.accept().unwrap();
            assert!(matches!(
                connection.send(&"x".repeat(MAX_MESSAGE_BYTES + 1)),
                Err(IpcError::MessageTooLarge)
            ));
        });
        let _client = connect(&name, 5_000).unwrap();
        worker.join().unwrap();
    }
}
