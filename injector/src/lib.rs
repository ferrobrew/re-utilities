use std::{os::windows::ffi::OsStrExt, path::Path};

use anyhow::Context;
use windows::{
    core::{s, w, Owned},
    Win32::{
        Foundation::HANDLE,
        System::{
            Diagnostics::{
                Debug::WriteProcessMemory,
                ToolHelp::{
                    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                    TH32CS_SNAPPROCESS,
                },
            },
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Memory::{
                VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            },
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
pub fn inject(process: HANDLE, payload_path: &Path) -> anyhow::Result<()> {
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
        WaitForSingleObject(*thread_handle, 5000);

        // Free memory
        VirtualFreeEx(process, alloc, 0, MEM_RELEASE).context("failed to free memory")?;
        WaitForSingleObject(process, INFINITE);
    }

    Ok(())
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
