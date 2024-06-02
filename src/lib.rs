use deno_runtime::deno_core;
use deno_core::{ModuleSpecifier,FeatureChecker};
use deno_core::error::AnyError;
use deno_runtime::worker::{MainWorker, WorkerOptions};
use deno_runtime::permissions::PermissionsContainer;
use deno_runtime::inspector_server::InspectorServer;
use deno_runtime::BootstrapOptions;
use deno_cache_dir::GlobalHttpCache;

use std::net::SocketAddr;
use std::path::{Path,PathBuf};
use std::sync::Arc;
use std::rc::Rc;

use velcro::vec;

use or_panic::OrPanic;

mod util;
use util::{FileFetcher,File,SJSModuleLoader,SJSCacheEnv,HttpClient,CacheSetting};
use util::ToAbsolutePath as _;

static CLI_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/CLI_SNAPSHOT.bin"));

#[derive(Clone)]
pub enum ScriptSource {
    File(String),
    Text(String),
    URL(String),
    FileOrURL(String)
}

pub struct InspectorOptions {
    pub port: Option<u16>,
    pub wait: bool
}

pub fn get_storage_directory() -> Option<PathBuf> {
    match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => Some(path.join(".sjs")),
        _ => None,
    }
}

pub async fn run(input: ScriptSource, args: Vec<String>, allow_remote: bool, inspector_options: InspectorOptions) -> Result<(), AnyError> {
    let source_name = match input.clone() {
        ScriptSource::File(source_path) => Path::new(&source_path).absolute().map(|x| String::from(x.into_os_string().into_string().unwrap())).map_err(|x| format!("{}: {}",source_path,x)).or_panic(),
        ScriptSource::URL(source_url) => source_url,
        ScriptSource::FileOrURL(source_path) => {
            Path::new(&source_path).absolute().map(|x| String::from(x.into_os_string().into_string().unwrap())).unwrap_or(source_path)
        }
        _ => String::new()
    };

    let sjs_storage_dir = get_storage_directory();

    let file_fetcher = FileFetcher::new(
        Arc::new(GlobalHttpCache::<SJSCacheEnv>::new(sjs_storage_dir.clone().unwrap_or_else(|| std::env::temp_dir()).join("libs"), SJSCacheEnv)),
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
            ModuleSpecifier::from_file_path(Path::new(&source_path).absolute().map_err(|x| format!("{}: {}",source_path,x)).or_panic().as_path()).unwrap()
        }
        ScriptSource::URL(source_url) => {
            ModuleSpecifier::parse(&source_url).or_panic()
        },
        ScriptSource::FileOrURL(source_path) => {
            Path::new(&source_path).absolute().map(|x| ModuleSpecifier::from_file_path(x.as_path()).unwrap()).unwrap_or_else(|_| ModuleSpecifier::parse(&source_path).map_err(|_x| format!("{}: {}",source_path,"Invalid file or URL")).or_panic())
        }
    };

    let inspector = match inspector_options.port {
        Some(port) => {
            let host = format!("127.0.0.1:{port}").parse::<SocketAddr>().unwrap();
            Some(Arc::new(InspectorServer::new(host, util::get_user_agent()).unwrap()))
        },
        _ => None
    };

    let unstable_features = (1..=8).collect::<Vec<i32>>(); // deno_runtime:lib.rs UNSTABLE_GRANULAR_FLAGS
    let feature_checker = {
        let mut feature_checker = FeatureChecker::default();
        feature_checker.set_exit_cb(Box::new(|_feature: &str, api_name: &str| {
            eprintln!("Unstable API '{api_name}' is not supported!");
            std::process::exit(70);
        }));

        for (flag_name, _, i) in deno_runtime::UNSTABLE_GRANULAR_FLAGS {
            if unstable_features.contains(i) {
                feature_checker.enable_feature(flag_name);
            }
        }

        Arc::new(feature_checker)
    };

    let mut worker = MainWorker::bootstrap_from_options(
        main_module.clone(),
        PermissionsContainer::allow_all(),
        WorkerOptions {
            bootstrap: BootstrapOptions {
                user_agent: util::get_user_agent().to_string(),
                args: vec![source_name, ..args],
                unstable_features,
                ..Default::default()
            },
            module_loader: Rc::new(SJSModuleLoader {file_fetcher}),
            extensions: vec![],
            startup_snapshot: Some(CLI_SNAPSHOT),
            // cache_storage_dir: std::env::temp_dir(),
            origin_storage_dir: sjs_storage_dir,

            should_wait_for_inspector_session: inspector_options.wait,
            maybe_inspector_server: inspector.clone(),
            
            feature_checker,

            ..Default::default()
        },
    );

    worker.js_runtime.maybe_init_inspector();
    worker.execute_main_module(&main_module).await?;
    worker.run_event_loop(false).await?;
    Ok(())
}