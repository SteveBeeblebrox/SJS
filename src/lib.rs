use std::path::Path;
use std::rc::Rc;

use deno_core::error::AnyError;
use deno_core::Snapshot;

use deno_runtime::deno_core::FsModuleLoader;
use deno_core::ModuleSpecifier;
use deno_runtime::permissions::PermissionsContainer;
use deno_runtime::worker::MainWorker;
use deno_runtime::worker::WorkerOptions;

static CLI_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/CLI_SNAPSHOT.bin"));

pub async fn run() -> Result<(), AnyError> {
    let js_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/main.js");
    let main_module = ModuleSpecifier::from_file_path(js_path).unwrap();
    let mut worker = MainWorker::bootstrap_from_options(
        main_module.clone(),
        PermissionsContainer::allow_all(),
        WorkerOptions {
            module_loader: Rc::new(FsModuleLoader),
            extensions: vec![],
            startup_snapshot: Some(Snapshot::Static(CLI_SNAPSHOT)),
            ..Default::default()
        },
    );

    worker.execute_main_module(&main_module).await?;
    worker.run_event_loop(false).await?;
    Ok(())
}