pub mod injector;
pub mod spawn;

use std::path::{Path, PathBuf};

use anyhow::Context;
use dll_syringe::process::{OwnedProcess, Process};

pub fn launch_and_inject(
    process_name: &str,
    spawner: impl Fn() -> anyhow::Result<OwnedProcess>,
    payload_name: &str,
    inject_into_running_process: bool,
) -> anyhow::Result<OwnedProcess> {
    let payload_path = std::env::current_exe()?
        .parent()
        .context("failed to find launcher executable directory")?
        .join(payload_name);

    let found_process = inject_into_running_process
        .then(|| process_name)
        .and_then(OwnedProcess::find_first_by_name);
    let process = match found_process {
        Some(process) => process,
        None => spawner()?,
    };
    let result = injector::inject(process.borrowed(), &payload_path);
    if result.is_err() {
        let _ = process.kill();
    }
    result.map(|_| process)
}

pub fn get_executable_name_from_builder(
    executable_path_builder: impl Fn(&Path) -> PathBuf + Copy,
) -> Option<String> {
    executable_path_builder(Path::new(""))
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .map(ToString::to_string)
}

pub fn launch_steam_process_and_inject(
    app_id: u32,
    executable_path_builder: impl Fn(&Path) -> PathBuf + Copy,
    payload_name: &str,
    inject_into_running_process: bool,
) -> anyhow::Result<OwnedProcess> {
    let process_name = get_executable_name_from_builder(executable_path_builder)
        .context("failed to get executable filename")?;

    launch_and_inject(
        &process_name,
        || spawn::steam_process(app_id, executable_path_builder),
        payload_name,
        inject_into_running_process,
    )
}

pub fn launch_env_path_process_and_inject(
    executable_path_builder: impl Fn(&Path) -> PathBuf + Copy,
    payload_name: &str,
    inject_into_running_process: bool,
) -> anyhow::Result<OwnedProcess> {
    let game_path: PathBuf = dunce::canonicalize(Path::new(
        &std::env::args()
            .nth(1)
            .context("failed to get path from args")?,
    ))?;
    let executable_path = executable_path_builder(&game_path);
    let process_name = executable_path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .context("failed to get filename")?
        .to_string();

    launch_and_inject(
        &process_name,
        || spawn::arbitrary_process(&game_path, &executable_path, std::iter::empty()),
        payload_name,
        inject_into_running_process,
    )
}
