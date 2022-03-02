use anyhow::Context;
use std::path::{Path, PathBuf};

use dll_syringe::process::OwnedProcess;
use windows::Win32::{Foundation::HANDLE, System::Threading};

pub struct ThreadResumer(HANDLE);
impl ThreadResumer {
    pub fn resume(&self) {
        unsafe {
            Threading::ResumeThread(self.0);
        }
    }
}

pub fn arbitrary_process(
    game_path: &Path,
    executable_path: &Path,
    env_vars: impl Iterator<Item = (String, String)>,
    create_suspended: bool,
) -> anyhow::Result<(OwnedProcess, Option<ThreadResumer>)> {
    use std::os::windows::prelude::FromRawHandle;

    let startup_info = Threading::STARTUPINFOW::default();
    let mut process_info = Threading::PROCESS_INFORMATION::default();

    let environment: Vec<u16> = std::env::vars()
        .chain(env_vars)
        .fold(String::new(), |a, (k, v)| format!("{}{}={}\0", a, k, v))
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut creation_flags = Threading::CREATE_UNICODE_ENVIRONMENT;
    if create_suspended {
        creation_flags |= Threading::CREATE_SUSPENDED;
    }

    unsafe {
        Threading::CreateProcessW(
            executable_path.as_os_str(),
            Default::default(),
            std::ptr::null(),
            std::ptr::null(),
            false,
            creation_flags,
            environment.as_ptr() as _,
            game_path.as_os_str(),
            &startup_info,
            &mut process_info,
        )
        .as_bool()
        .then(|| {
            (
                OwnedProcess::from_raw_handle(process_info.hProcess.0 as _),
                Some(ThreadResumer(process_info.hThread)),
            )
        })
        .context("failed to spawn process")
    }
}

pub fn steam_process(
    app_id: u32,
    executable_path_builder: fn(&Path) -> PathBuf,
    create_suspended: bool,
) -> anyhow::Result<(OwnedProcess, Option<ThreadResumer>)> {
    let game_path = steamlocate::SteamDir::locate()
        .context("failed to locate steamdir")?
        .app(&app_id)
        .context("failed to locate app")?
        .path
        .clone();
    let executable_path = executable_path_builder(&game_path);

    let env_vars = ["SteamGameId", "SteamAppId"]
        .iter()
        .map(|s| (s.to_string(), app_id.to_string()));

    arbitrary_process(&game_path, &executable_path, env_vars, create_suspended)
}
