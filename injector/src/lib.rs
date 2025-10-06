use std::{
    ffi::OsString,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    ptr::NonNull,
};

use anyhow::Context;
use windows::{
    core::{s, w, Owned, HSTRING, PCSTR},
    Win32::{
        Foundation::{HANDLE, HMODULE, MAX_PATH, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            Diagnostics::{
                Debug::WriteProcessMemory,
                ToolHelp::{
                    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW,
                    Process32NextW, MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE,
                    TH32CS_SNAPPROCESS,
                },
            },
            LibraryLoader::{
                GetModuleHandleW, GetProcAddress, LoadLibraryExW, DONT_RESOLVE_DLL_REFERENCES,
            },
            Memory::{
                VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            },
            ProcessStatus::GetModuleFileNameExW,
            Threading::{
                CreateRemoteThread, OpenProcess, WaitForSingleObject, INFINITE,
                PROCESS_CREATE_THREAD, PROCESS_TERMINATE, PROCESS_VM_OPERATION, PROCESS_VM_READ,
                PROCESS_VM_WRITE,
            },
        },
    },
};

pub mod spawn;

/// Injects a DLL into a process. To get a process handle, use [`get_processes_by_name`] or
/// functions from [`spawn`].
///
/// Note that this will only work when injecting into a process of the same architecture as the
/// injector. For example, a 64-bit injector can only inject into a 64-bit process.
///
/// Returns the path to the injected DLL.
pub fn inject(process: HANDLE, payload_path: &Path) -> anyhow::Result<PathBuf> {
    let injected_payload_path = {
        let decompose_filename = |filename: &Path| {
            Some((
                filename.file_stem()?.to_str()?.to_owned(),
                filename.extension()?.to_str()?.to_owned(),
            ))
        };

        let (stem, extension) =
            decompose_filename(payload_path).context("failed to decompose filename")?;

        let injected_payload_filename = Path::new(&(stem + "_loaded")).with_extension(extension);
        payload_path.with_file_name(&injected_payload_filename)
    };

    // inject
    if !injected_payload_path.exists()
        || std::fs::read(payload_path)? != std::fs::read(&injected_payload_path)?
    {
        std::fs::copy(payload_path, &injected_payload_path)?;
    }

    let dll_path: Vec<u16> = injected_payload_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        // Allocate memory in the target process
        let alloc = VirtualAllocEx(
            process,
            None,
            dll_path.len() * std::mem::size_of::<u16>(),
            MEM_RESERVE | MEM_COMMIT,
            PAGE_EXECUTE_READWRITE,
        );
        if alloc.is_null() {
            anyhow::bail!(
                "failed to allocate memory in remote process: {:?}",
                windows::core::Error::from_win32()
            );
        }

        // Write the DLL path to the target process
        let mut bytes_written = 0;
        WriteProcessMemory(
            process,
            alloc,
            dll_path.as_ptr() as *const _,
            dll_path.len() * std::mem::size_of::<u16>(),
            Some(&mut bytes_written),
        )
        .context("failed to write memory")?;

        // Get the address of LoadLibraryW
        let kernel32_module =
            GetModuleHandleW(w!("kernel32.dll")).context("failed to get module")?;
        let load_library = GetProcAddress(kernel32_module, s!("LoadLibraryW"));
        let Some(load_library) = load_library else {
            anyhow::bail!(
                "failed to get LoadLibraryW address: {:?}",
                windows::core::Error::from_win32()
            );
        };

        // Create a remote thread to load the DLL
        #[allow(clippy::missing_transmute_annotations)]
        let thread_handle = Owned::new(
            CreateRemoteThread(
                process,
                None,
                0,
                Some(std::mem::transmute(load_library)),
                Some(alloc),
                0,
                None,
            )
            .context("failed to create remote thread")?,
        );

        // Wait for thread to finish
        let result = WaitForSingleObject(*thread_handle, 5000);
        if result == WAIT_ABANDONED || result == WAIT_TIMEOUT || result.0 == INFINITE {
            anyhow::bail!("failed to inject DLL: {result:?}");
        }

        // Free memory
        VirtualFreeEx(process, alloc, 0, MEM_RELEASE).context("failed to free memory")?;
    }

    Ok(injected_payload_path)
}

/// Calls a remote export of a module in a remote process.
pub fn call_remote_export(
    process: HANDLE,
    remote_module_base: NonNull<u8>,
    export_name: &str,
) -> anyhow::Result<()> {
    unsafe {
        // Get the module file name from the remote process
        let mut module_path = [0u16; MAX_PATH as usize];
        let result = GetModuleFileNameExW(
            process,
            HMODULE(remote_module_base.as_ptr() as *mut std::ffi::c_void),
            &mut module_path,
        );
        if result == 0 {
            anyhow::bail!(
                "Failed to get remote export call module path: {:?}",
                windows::core::Error::from_win32()
            );
        }
        let module_path = OsString::from_wide(&module_path);

        // Load the module locally with DONT_RESOLVE_DLL_REFERENCES
        // This loads and maps the module into memory, but does not load any
        // dependencies or call any code in the module
        let local_module = LoadLibraryExW(
            &HSTRING::from(module_path),
            None,
            DONT_RESOLVE_DLL_REFERENCES,
        )
        .context("failed to load library")?;

        // Get the address of the export in our local copy of the DLL
        let export_name_cstr =
            std::ffi::CString::new(export_name).context("Invalid export name")?;
        let local_addr =
            GetProcAddress(local_module, PCSTR(export_name_cstr.as_ptr() as *const u8));
        let Some(local_addr) = local_addr else {
            anyhow::bail!(
                "Failed to locate remote call export: {:?}",
                windows::core::Error::from_win32()
            );
        };

        // Calculate the remote address by subtracting the local module base
        // and adding the remote module base
        let local_module_base = local_module.0 as *const u8;
        let local_addr_ptr = local_addr as *const u8;
        let offset = local_addr_ptr as usize - local_module_base as usize;
        let remote_addr = (remote_module_base.as_ptr() as usize + offset) as *const u8;

        // Create a new thread at the calculated function address
        let thread_handle = Owned::new(
            CreateRemoteThread(
                process,
                None,
                0,
                #[allow(clippy::missing_transmute_annotations)]
                Some(std::mem::transmute(remote_addr)),
                None,
                0,
                None,
            )
            .context("Failed to create remote call thread")?,
        );

        // Wait for the thread to complete (10 second timeout)
        let result = WaitForSingleObject(*thread_handle, 10000);

        match result {
            WAIT_OBJECT_0 => Ok(()),
            WAIT_TIMEOUT => {
                anyhow::bail!("Remote call thread timed out");
            }
            WAIT_ABANDONED => {
                anyhow::bail!("Remote call thread was abandoned");
            }
            _ => {
                anyhow::bail!("Waiting for remote call thread failed: {}", result.0);
            }
        }
    }
}

/// Returns the base address of a module in a remote process.
pub fn get_remote_module_base(
    process_id: u32,
    module_path: &Path,
) -> anyhow::Result<Option<NonNull<u8>>> {
    let th = unsafe {
        Owned::new(
            CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, process_id)
                .context("failed to create snapshot")?,
        )
    };

    let mut entry = MODULEENTRY32W {
        dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
        ..Default::default()
    };

    let module_path = module_path
        .canonicalize()
        .context("failed to canonicalize module path")?;

    unsafe {
        Module32FirstW(*th, &mut entry).context("failed to get first module")?;

        loop {
            let len = entry
                .szExePath
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExePath.len());

            if PathBuf::from(&*OsString::from_wide(&entry.szExePath[..len])).canonicalize()?
                == module_path
            {
                return Ok(NonNull::new(entry.modBaseAddr));
            }

            if Module32NextW(*th, &mut entry).is_err() {
                break;
            }
        }
    }

    Ok(None)
}

/// Gets a list of process handles by their name, if running.
pub fn get_processes_by_name(name: &str) -> windows::core::Result<Vec<(u32, Owned<HANDLE>)>> {
    unsafe {
        let snapshot = Owned::new(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?);
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut handles = Vec::new();

        if Process32FirstW(*snapshot, &mut entry).is_ok() {
            loop {
                let process_name = String::from_utf16_lossy(&entry.szExeFile)
                    .trim_end_matches('\0')
                    .to_lowercase();

                if process_name == name.to_lowercase() {
                    if let Ok(handle) = OpenProcess(
                        PROCESS_VM_READ
                            | PROCESS_VM_WRITE
                            | PROCESS_VM_OPERATION
                            | PROCESS_TERMINATE
                            | PROCESS_CREATE_THREAD,
                        false,
                        entry.th32ProcessID,
                    ) {
                        handles.push((entry.th32ProcessID, Owned::new(handle)));
                    }
                }

                if Process32NextW(*snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        Ok(handles)
    }
}
