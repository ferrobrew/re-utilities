pub mod injector;
pub mod spawn;

use std::path::{Path, PathBuf};

use anyhow::Context;
use dll_syringe::process::{OwnedProcess, Process};

pub fn launch_and_inject(
    process_name: &str,
    spawner: impl Fn() -> anyhow::Result<(OwnedProcess, Option<spawn::ThreadResumer>)>,
    payload_name: &str,
    resume_after_injection: bool,
    inject_into_running_process: bool,
) -> anyhow::Result<()> {
    let payload_path = std::env::current_exe()?
        .parent()
        .context("failed to find launcher executable directory")?
        .join(payload_name);

    let found_process = if inject_into_running_process {
        OwnedProcess::find_first_by_name(process_name)
    } else {
        None
    };
    let (process, main_thread) = match found_process {
        Some(process) => (process, None),
        None => spawner()?,
    };
    let result = injector::inject(process.borrowed(), &payload_path);
    match (result.is_ok(), resume_after_injection, main_thread) {
        (true, true, Some(main_thread)) => {
            main_thread.resume();
        }
        (false, ..) => {
            let _ = process.kill();
        }
        _ => {}
    }
    result
}

pub fn launch_steam_process_and_inject(
    app_id: u32,
    executable_path_builder: fn(&Path) -> PathBuf,
    payload_name: &str,
    resume_after_injection: bool,
    inject_into_running_process: bool,
) -> anyhow::Result<()> {
    let process_name = executable_path_builder(Path::new(""))
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .context("failed to get executable filename")?
        .to_owned();

    launch_and_inject(
        &process_name,
        || spawn::steam_process(app_id, executable_path_builder, true),
        payload_name,
        resume_after_injection,
        inject_into_running_process,
    )
}
