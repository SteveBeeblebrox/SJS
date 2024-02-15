use std::path::Path;
use std::rc::Rc;

use deno_core::error::AnyError;
use deno_core::Snapshot;

use deno_runtime::deno_core::FsModuleLoader;
use deno_core::ModuleSpecifier;
use deno_runtime::permissions::PermissionsContainer;
use deno_runtime::worker::MainWorker;
use deno_runtime::worker::WorkerOptions;
use deno_runtime::BootstrapOptions;

use velcro::vec;

static CLI_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/CLI_SNAPSHOT.bin"));

pub enum ScriptSource {
    File(String),
    Text(String)
}

pub async fn run(input: ScriptSource, args: Vec<String>) -> Result<(), AnyError> {
    let runtime_version = env!("CARGO_PKG_VERSION");
    let user_agent = format!("sjs/{runtime_version}");

    let source_name = match input {
        ScriptSource::File(path) => path,
        _ => String::new()
    };

    let js_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/main.js");
    let main_module = ModuleSpecifier::from_file_path(js_path).unwrap();
    let mut worker = MainWorker::bootstrap_from_options(
        main_module.clone(),
        PermissionsContainer::allow_all(),
        WorkerOptions {
            bootstrap: BootstrapOptions {
                user_agent,
                args: vec![source_name, ..args],
                ..Default::default()
            },
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