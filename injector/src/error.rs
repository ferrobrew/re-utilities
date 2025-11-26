use std::fmt;

/// Windows API errors
#[derive(Debug)]
pub enum WindowsError {
    /// Failed to allocate memory in the remote process
    RemoteMemoryAllocation { source: windows::core::Error },
    /// Failed to write to process memory
    WriteMemory { source: windows::core::Error },
    /// Failed to get module handle
    GetModuleHandle {
        module: String,
        source: windows::core::Error,
    },
    /// Failed to get procedure address
    GetProcAddress {
        procedure: String,
        source: windows::core::Error,
    },
    /// Failed to create a remote thread
    CreateRemoteThread {
        context: String,
        source: windows::core::Error,
    },
    /// DLL injection wait failed
    InjectionWaitFailed { result: u32 },
    /// Failed to free remote memory
    FreeMemory { source: windows::core::Error },
    /// Failed to get module file name from remote process
    GetRemoteModuleFileName { source: windows::core::Error },
    /// Failed to load library
    LoadLibrary { source: windows::core::Error },
    /// Failed to locate export in module
    ExportNotFound {
        export_name: String,
        source: windows::core::Error,
    },
    /// Remote call thread timed out
    RemoteCallTimeout,
    /// Remote call thread was abandoned
    RemoteCallAbandoned,
    /// Waiting for remote call thread failed
    RemoteCallWaitFailed { result: u32 },
    /// Failed to create a toolhelp snapshot
    CreateSnapshot { source: windows::core::Error },
    /// Failed to get first module from snapshot
    GetFirstModule { source: windows::core::Error },
    /// Failed to spawn a process
    SpawnProcess { source: windows::core::Error },
}

impl fmt::Display for WindowsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowsError::RemoteMemoryAllocation { source } => {
                write!(f, "failed to allocate memory in remote process: {}", source)
            }
            WindowsError::WriteMemory { source } => {
                write!(f, "failed to write memory: {}", source)
            }
            WindowsError::GetModuleHandle { module, source } => {
                write!(
                    f,
                    "failed to get handle for module '{}': {}",
                    module, source
                )
            }
            WindowsError::GetProcAddress { procedure, source } => {
                write!(
                    f,
                    "failed to get address for procedure '{}': {}",
                    procedure, source
                )
            }
            WindowsError::CreateRemoteThread { context, source } => {
                write!(
                    f,
                    "failed to create remote thread ({}): {}",
                    context, source
                )
            }
            WindowsError::InjectionWaitFailed { result } => {
                write!(f, "failed to inject DLL: wait returned {}", result)
            }
            WindowsError::FreeMemory { source } => {
                write!(f, "failed to free memory: {}", source)
            }
            WindowsError::GetRemoteModuleFileName { source } => {
                write!(f, "failed to get remote module file name: {}", source)
            }
            WindowsError::LoadLibrary { source } => {
                write!(f, "failed to load library: {}", source)
            }
            WindowsError::ExportNotFound {
                export_name,
                source,
            } => {
                write!(f, "failed to locate export '{}': {}", export_name, source)
            }
            WindowsError::RemoteCallTimeout => {
                write!(f, "remote call thread timed out")
            }
            WindowsError::RemoteCallAbandoned => {
                write!(f, "remote call thread was abandoned")
            }
            WindowsError::RemoteCallWaitFailed { result } => {
                write!(f, "waiting for remote call thread failed: {}", result)
            }
            WindowsError::CreateSnapshot { source } => {
                write!(f, "failed to create snapshot: {}", source)
            }
            WindowsError::GetFirstModule { source } => {
                write!(f, "failed to get first module: {}", source)
            }
            WindowsError::SpawnProcess { source } => {
                write!(f, "failed to spawn process: {}", source)
            }
        }
    }
}

impl std::error::Error for WindowsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WindowsError::RemoteMemoryAllocation { source } => Some(source),
            WindowsError::WriteMemory { source } => Some(source),
            WindowsError::GetModuleHandle { source, .. } => Some(source),
            WindowsError::GetProcAddress { source, .. } => Some(source),
            WindowsError::CreateRemoteThread { source, .. } => Some(source),
            WindowsError::FreeMemory { source } => Some(source),
            WindowsError::GetRemoteModuleFileName { source } => Some(source),
            WindowsError::LoadLibrary { source } => Some(source),
            WindowsError::ExportNotFound { source, .. } => Some(source),
            WindowsError::CreateSnapshot { source } => Some(source),
            WindowsError::GetFirstModule { source } => Some(source),
            WindowsError::SpawnProcess { source } => Some(source),
            _ => None,
        }
    }
}

/// Steam-related errors
#[derive(Debug)]
pub enum SteamError {
    /// Failed to locate Steam app
    AppNotFound { app_id: u32 },
    /// Steam locate error
    Locate { source: steamlocate::Error },
}

impl fmt::Display for SteamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SteamError::AppNotFound { app_id } => {
                write!(f, "failed to locate Steam app {}", app_id)
            }
            SteamError::Locate { source } => {
                write!(f, "Steam locate error: {}", source)
            }
        }
    }
}

impl std::error::Error for SteamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SteamError::Locate { source } => Some(source),
            _ => None,
        }
    }
}

/// Error type for re-utilities-injector operations
#[derive(Debug)]
pub enum Error {
    /// Failed to decompose the filename from the payload path
    FilenameDecomposition,
    /// I/O operation failed
    Io {
        context: Option<String>,
        source: std::io::Error,
    },
    /// Invalid export name (contains null byte)
    InvalidExportName { source: std::ffi::NulError },
    /// Failed to canonicalize a path
    CanonicalizePath { source: std::io::Error },
    /// Windows API error
    Windows(WindowsError),
    /// Steam-related error
    Steam(SteamError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::FilenameDecomposition => {
                write!(f, "failed to decompose filename from payload path")
            }
            Error::Io { context, source } => {
                if let Some(ctx) = context {
                    write!(f, "{}: {}", ctx, source)
                } else {
                    write!(f, "I/O error: {}", source)
                }
            }
            Error::InvalidExportName { source } => {
                write!(f, "invalid export name: {}", source)
            }
            Error::CanonicalizePath { source } => {
                write!(f, "failed to canonicalize module path: {}", source)
            }
            Error::Windows(e) => write!(f, "{}", e),
            Error::Steam(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::InvalidExportName { source } => Some(source),
            Error::CanonicalizePath { source } => Some(source),
            Error::Windows(e) => e.source(),
            Error::Steam(e) => e.source(),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Error::Io {
            context: None,
            source,
        }
    }
}

impl From<std::ffi::NulError> for Error {
    fn from(source: std::ffi::NulError) -> Self {
        Error::InvalidExportName { source }
    }
}

impl From<steamlocate::Error> for Error {
    fn from(source: steamlocate::Error) -> Self {
        Error::Steam(SteamError::Locate { source })
    }
}

impl From<WindowsError> for Error {
    fn from(source: WindowsError) -> Self {
        Error::Windows(source)
    }
}

impl From<SteamError> for Error {
    fn from(source: SteamError) -> Self {
        Error::Steam(source)
    }
}

/// Result type alias for re-utilities-injector operations
pub type Result<T> = std::result::Result<T, Error>;
