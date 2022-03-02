use std::path::Path;

use anyhow::Context;
use dll_syringe::{
    process::{BorrowedProcess, BorrowedProcessModule, ProcessModule},
    Syringe,
};

pub fn call_procedure(
    syringe: &Syringe,
    process_module: BorrowedProcessModule,
    procedure_name: &str,
) -> anyhow::Result<u64> {
    let procedure = syringe
        .get_procedure(process_module, procedure_name)?
        .context(format!("failed to find function: {}", procedure_name))?;

    Ok(procedure.call(&0_u64)?)
}

pub fn inject(process: BorrowedProcess, payload_path: &Path) -> anyhow::Result<()> {
    let syringe = Syringe::for_process(process.try_to_owned()?);

    let injected_payload_path = {
        let decompose_filename = |filename: &Path| {
            Some((
                filename.file_stem()?.to_str()?.to_owned(),
                filename.extension()?.to_str()?.to_owned(),
            ))
        };

        let (stem, extension) =
            decompose_filename(payload_path).context("failed to decompose filename")?;

        payload_path
            .with_file_name(format!("{}_loaded", stem))
            .with_extension(extension)
    };

    // eject
    if let Some(process_module) = ProcessModule::find_by_name(&injected_payload_path, process)? {
        call_procedure(&syringe, process_module, "unload")?;
        syringe.eject(process_module)?;
    }

    // inject
    std::fs::copy(&payload_path, &injected_payload_path)?;
    let process_module = syringe.inject(&injected_payload_path)?;
    call_procedure(&syringe, process_module, "load")?;

    // Would ideally return some form of ProcessModule here, but need to figure out borrows:
    // https://github.com/OpenByteDev/dll-syringe/issues/1
    Ok(())
}
