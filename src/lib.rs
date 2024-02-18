use deno_core::{Snapshot,ModuleSpecifier};
use deno_core::error::AnyError;
use deno_runtime::worker::{MainWorker, WorkerOptions};
use deno_runtime::permissions::PermissionsContainer;
use deno_runtime::BootstrapOptions;
use deno_cache_dir::GlobalHttpCache;

use std::path::{Path,PathBuf};
use std::sync::Arc;
use std::rc::Rc;

use velcro::vec;

use or_panic::OrPanic;

mod util;
use util::{FileFetcher,File,SJSModuleLoader,SJSCacheEnv,HttpClient};
use util::CacheSetting;

static CLI_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/CLI_SNAPSHOT.bin"));

#[derive(Clone)]
pub enum ScriptSource {
    File(String),
    Text(String),
    URL(String),
    FileOrURL(String)
}


pub fn get_storage_directory() -> Option<PathBuf> {
    match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => Some(path.join(".sjs")),
        _ => None,
    }
}

pub async fn run(input: ScriptSource, args: Vec<String>, allow_remote: bool) -> Result<(), AnyError> {
    let source_name = match input.clone() {
        ScriptSource::File(source_path) => Path::new(&source_path).canonicalize().map(|x| String::from(x.into_os_string().into_string().unwrap())).map_err(|x| format!("{}: {}",source_path,x)).or_panic(),
        ScriptSource::URL(source_url) => source_url,
        ScriptSource::FileOrURL(source_path) => {
            Path::new(&source_path).canonicalize().map(|x| String::from(x.into_os_string().into_string().unwrap())).unwrap_or(source_path)
        }
        _ => String::new()
    };

    let sjs_storage_dir = get_storage_directory();

    let file_fetcher = FileFetcher::new(
        Arc::new(GlobalHttpCache::<SJSCacheEnv>::new(sjs_storage_dir.clone().unwrap_or(std::env::temp_dir()).join("libs"), SJSCacheEnv)),
        CacheSetting::Use,
        allow_remote,
        Arc::new(HttpClient::new(Default::default(),None)),
        Default::default(),
    );

    let main_module = match input {
        ScriptSource::Text(source_text) => {
            let main_module = ModuleSpecifier::parse("sjs://text").unwrap();
            let bytes: Vec<u8> = source_text.into();
            file_fetcher.insert_cached(File {
                specifier: main_module.clone(),
                maybe_headers: None,
                source: bytes.into(),
            });
            main_module
        }
        ScriptSource::File(source_path) => {
            ModuleSpecifier::from_file_path(Path::new(&source_path).canonicalize().map_err(|x| format!("{}: {}",source_path,x)).or_panic().as_path()).unwrap()
        }
        ScriptSource::URL(source_url) => {
            ModuleSpecifier::parse(&source_url).or_panic()
        },
        ScriptSource::FileOrURL(source_path) => {
            Path::new(&source_path).canonicalize().map(|x| ModuleSpecifier::from_file_path(x.as_path()).unwrap()).unwrap_or(ModuleSpecifier::parse(&source_path).map_err(|_x| format!("{}: {}",source_path,"Invalid file or URL")).or_panic())
        }
    };

    let mut worker = MainWorker::bootstrap_from_options(
        main_module.clone(),
        PermissionsContainer::allow_all(),
        WorkerOptions {
            bootstrap: BootstrapOptions {
                user_agent: util::get_user_agent(),
                args: vec![source_name, ..args],
                ..Default::default()
            },
            module_loader: Rc::new(SJSModuleLoader {file_fetcher}),
            extensions: vec![],
            startup_snapshot: Some(Snapshot::Static(CLI_SNAPSHOT)),
            // cache_storage_dir: std::env::temp_dir(),
            origin_storage_dir: sjs_storage_dir,
            ..Default::default()
        },
    );

    worker.execute_main_module(&main_module).await?;
    worker.run_event_loop(false).await?;
    Ok(())
}