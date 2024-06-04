#![allow(unused_imports,unused_variables,dead_code)] // Temp

use deno_runtime::deno_core;
use deno_core::{ModuleSpecifier,FeatureChecker};
use deno_core::error::AnyError;
use deno_runtime::worker::{MainWorker, WorkerOptions};
use deno_runtime::permissions::PermissionsContainer;
use deno_runtime::inspector_server::InspectorServer;
use deno_runtime::{BootstrapOptions, WorkerExecutionMode};
use deno_cache_dir::GlobalHttpCache;

use deno_runtime::deno_broadcast_channel::{BroadcastChannel, InMemoryBroadcastChannel};

use std::net::SocketAddr;
use std::path::{Path,PathBuf};
use std::sync::Arc;
use std::rc::Rc;

use velcro::vec;

use or_panic::OrPanic;

mod util;
use util::{FileFetcher,File,SJSModuleLoader,SJSCacheEnv,HttpClient,CacheSetting};
use util::ToAbsolutePath as _;

static STARTUP_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/STARTUP_SNAPSHOT.bin"));

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

use deno_core::{
    SharedArrayBufferStore,
    CompiledWasmModuleStore,
};

use deno_runtime::deno_web::{
    BlobStore
};


#[derive(Clone)]
pub struct SharedState {
    args: Vec<String>,

    broadcast_channel: InMemoryBroadcastChannel,

    shared_array_buffer_store: Option<SharedArrayBufferStore>,
    compiled_wasm_module_store: Option<CompiledWasmModuleStore>,
    blob_store: Arc<BlobStore>,

    seed: Option<u64>,

    unstable_features: Vec<i32>,

    inspector: Option<Arc<InspectorServer>>,

    file_fetcher: Arc<FileFetcher>
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            args: vec![],

            broadcast_channel: Default::default(),

            shared_array_buffer_store: Some(Default::default()),
            compiled_wasm_module_store: Some(Default::default()),
            blob_store: Default::default(),

            seed: None,

            // See deno_runtime::UNSTABLE_GRANULAR_FLAGS
            unstable_features: (1..=8).collect(),

            inspector: None,

            file_fetcher: Arc::new(FileFetcher::new(
                Arc::new(GlobalHttpCache::<SJSCacheEnv>::new(get_temp_directory().join("libs"), SJSCacheEnv)),
                CacheSetting::Use,
                false,
                Arc::new(HttpClient::new(Default::default(),None)),
                Default::default(),
            ))
        }
    }
}

fn create_feature_checker(unstable_features: &Vec<i32>) -> Arc<FeatureChecker> {
    let mut feature_checker = FeatureChecker::default();
    feature_checker.set_exit_cb(Box::new(|_feature: &str, api_name: &str| {
        panic!("Unstable API '{api_name}' is not supported!");
    }));

    for (flag_name, _, i) in deno_runtime::UNSTABLE_GRANULAR_FLAGS {
        if unstable_features.contains(i) {
            feature_checker.enable_feature(flag_name);
        }
    }

    Arc::new(feature_checker)
}


fn create_web_worker_callback(shared: Arc<SharedState>) -> Arc<deno_runtime::ops::worker_host::CreateWebWorkerCb> {
    Arc::new(move |args| {
        use deno_runtime::web_worker::{WebWorker,WebWorkerOptions};

        let options = WebWorkerOptions {
            bootstrap: BootstrapOptions {
                args: vec![args.main_module.to_string(), ..shared.args.clone()],
                location: Some(args.main_module.clone()),
                unstable_features: shared.unstable_features.clone(),
                user_agent: util::get_user_agent().to_string(),
                inspect: shared.inspector.is_some(),
                future: true,
                mode: WorkerExecutionMode::Worker,

                ..Default::default()
            },
            extensions: vec![],
            startup_snapshot: Some(STARTUP_SNAPSHOT),
            unsafely_ignore_certificate_errors: None,
            root_cert_store_provider:                                                             None,  // TODO Option<Arc<dyn RootCertStoreProvider>>
            seed: shared.seed,
            fs: Arc::new(deno_runtime::deno_fs::RealFs),
            module_loader: Rc::new(SJSModuleLoader {file_fetcher: shared.file_fetcher.clone()}),
            npm_resolver: None,
            create_web_worker_cb: create_web_worker_callback(shared.clone()),
            format_js_error_fn: Some(Arc::new(deno_runtime::fmt_errors::format_js_error)),
            source_map_getter: None, // Source maps not implemented, may change in future
            worker_type: args.worker_type,
            maybe_inspector_server: shared.inspector.clone(),
            get_error_class_fn: Some(&(|e| deno_runtime::errors::get_error_class_name(e).unwrap_or("Error"))),
            blob_store: shared.blob_store.clone(),
            broadcast_channel: shared.broadcast_channel.clone(),
            shared_array_buffer_store: shared.shared_array_buffer_store.clone(),
            compiled_wasm_module_store: shared.compiled_wasm_module_store.clone(),
            cache_storage_dir: Some(get_temp_directory().join(util::hash(&[args.main_module.to_string().as_bytes()]))),
            stdio: Default::default(),
            feature_checker: create_feature_checker(&shared.unstable_features),
            strace_ops: None,
            close_on_idle: args.close_on_idle,
            maybe_worker_metadata: args.maybe_worker_metadata
        };

        WebWorker::bootstrap_from_options(
            args.name,
            args.permissions,
            args.main_module,
            args.worker_id,
            options
        )
    })
}

pub fn get_storage_directory() -> Option<PathBuf> {
    match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => Some(path.join(".sjs")),
        _ => None,
    }
}

pub fn get_temp_directory() -> PathBuf {
    std::env::temp_dir().join("sjs")
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

    let file_fetcher = Arc::new(FileFetcher::new(
        Arc::new(GlobalHttpCache::<SJSCacheEnv>::new(get_storage_directory().unwrap_or_else(|| get_temp_directory()).join("libs"), SJSCacheEnv)),
        CacheSetting::Use,
        allow_remote,
        Arc::new(HttpClient::new(Default::default(),None)),
        Default::default(),
    ));

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

    let shared = Arc::new(SharedState {
        args,
        inspector: match inspector_options.port {
            Some(port) => {
                let host = format!("127.0.0.1:{port}").parse::<SocketAddr>().unwrap();
                Some(Arc::new(InspectorServer::new(host, util::get_user_agent()).unwrap()))
            },
            _ => None
        },
        file_fetcher,

        ..Default::default()
    });


    let options = WorkerOptions {
        bootstrap: BootstrapOptions {
            args: vec![main_module.to_string(), ..shared.args.clone()],
            location: Some(main_module.clone()),
            unstable_features: shared.unstable_features.clone(),
            user_agent: util::get_user_agent().to_string(),
            inspect: shared.inspector.is_some(),
            future: true,
            mode: WorkerExecutionMode::None,

            ..Default::default()
        },
        extensions: vec![],
        startup_snapshot: Some(STARTUP_SNAPSHOT),
        skip_op_registration: false,
        create_params: None,
        unsafely_ignore_certificate_errors: None,
        root_cert_store_provider:                                                             None,  // TODO Option<Arc<dyn RootCertStoreProvider>>
        seed: shared.seed,
        fs: Arc::new(deno_runtime::deno_fs::RealFs),
        module_loader: Rc::new(SJSModuleLoader {file_fetcher: shared.file_fetcher.clone()}),
        npm_resolver: None,
        create_web_worker_cb: create_web_worker_callback(shared.clone()),
        format_js_error_fn: Some(Arc::new(deno_runtime::fmt_errors::format_js_error)),
        source_map_getter: None, // Source maps not implemented, may change in future
        maybe_inspector_server: shared.inspector.clone(),
        should_break_on_first_statement: false,
        should_wait_for_inspector_session: inspector_options.wait,
        strace_ops: None,
        get_error_class_fn: Some(&(|e| deno_runtime::errors::get_error_class_name(e).unwrap_or("Error"))),
        cache_storage_dir: Some(get_temp_directory().join(util::hash(&[main_module.to_string().as_bytes()]))),
        
        origin_storage_dir: Some(get_storage_directory().unwrap_or_else(|| get_temp_directory()).join(util::hash(&[main_module.to_string().as_bytes()]))),

        blob_store: shared.blob_store.clone(),
        broadcast_channel: shared.broadcast_channel.clone(),
        shared_array_buffer_store: shared.shared_array_buffer_store.clone(),
        compiled_wasm_module_store: shared.compiled_wasm_module_store.clone(),
        stdio: Default::default(),
        feature_checker: create_feature_checker(&shared.unstable_features),
        v8_code_cache: None // // TODO implement source code cache? Option<Arc<dyn CodeCache>>,
    };

    let mut worker = MainWorker::bootstrap_from_options(
        main_module.clone(),
        PermissionsContainer::allow_all(),
        options
    );

    worker.js_runtime.maybe_init_inspector();
    worker.execute_main_module(&main_module).await?;
    worker.run_event_loop(false).await?;
    Ok(())
}