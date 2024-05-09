use anyhow::Context;
use std::path::{Path, PathBuf};

use windows::{core::HSTRING, Win32::System::Threading};

pub fn arbitrary_process(
    game_path: &Path,
    executable_path: &Path,
    env_vars: impl Iterator<Item = (String, String)>,
    create_suspended: bool,
) -> anyhow::Result<Threading::PROCESS_INFORMATION> {
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
        let application_name = HSTRING::from(executable_path.as_os_str());
        let current_directory = HSTRING::from(game_path.as_os_str());
        Threading::CreateProcessW(
            &application_name,
            windows::core::PWSTR::null(),
            std::ptr::null(),
            std::ptr::null(),
            false,
            creation_flags,
            environment.as_ptr() as _,
            &current_directory,
            &startup_info,
            &mut process_info,
        )
        .as_bool()
        .then(|| process_info)
        .context("failed to spawn process")
    }
}

pub fn steam_process(
    app_id: u32,
    executable_path_builder: impl Fn(&Path) -> PathBuf + Copy,
    create_suspended: bool,
) -> anyhow::Result<Threading::PROCESS_INFORMATION> {
    let steam_dir = steamlocate::SteamDir::locate()?;

    let (app, library) = steam_dir
        .find_app(app_id)?
        .context("failed to locate app")?;
    let game_path = library.resolve_app_dir(&app);
    let executable_path = executable_path_builder(&game_path);

    let env_vars = ["SteamGameId", "SteamAppId"]
        .iter()
        .map(|s| (s.to_string(), app_id.to_string()));

    arbitrary_process(&game_path, &executable_path, env_vars, create_suspended)
}
