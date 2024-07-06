use deno_runtime::deno_core;
use deno_core::{
  ModuleSpecifier, ModuleSource, ModuleType, RequestedModuleType, ResolutionKind,
  resolve_import, ModuleSourceCode, ModuleLoader, ModuleLoadResponse
};
use deno_core::anyhow::Error;
use deno_core::error::generic_error;
use deno_runtime::deno_permissions::PermissionsContainer;
use deno_core::futures::FutureExt;
use import_map::ImportMap;
use data_url::DataUrl;

use mtsc::util::OptionSource;

use std::path::PathBuf;
use std::sync::Arc;

use crate::util::{self, FileFetcher};

pub struct SJSModuleLoader {
  pub file_fetcher: Arc<FileFetcher>,
  pub macros: Vec<String>,
  pub include_paths: Vec<String>,
  pub import_map: Option<ImportMap>,
}

impl ModuleLoader for SJSModuleLoader {
    fn resolve(
      &self,
      specifier: &str,
      referrer: &str,
      _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, Error> {
      return match &self.import_map {
        Some(import_map) => Ok(import_map.resolve(specifier, &util::url::resolve_maybe_url(referrer)?)?),
        None => Ok(resolve_import(specifier, referrer)?)
      };
    }
  
    fn load(
      &self,
      module_specifier: &ModuleSpecifier,
      _maybe_referrer: Option<&ModuleSpecifier>,
      _is_dynamic: bool,
      requested_module_type: RequestedModuleType,
    ) -> ModuleLoadResponse {
      let module_specifier = module_specifier.clone();
      let file_fetcher = self.file_fetcher.clone();

      let macros = self.macros.clone();
      let include_paths = self.include_paths.clone();

      return ModuleLoadResponse::Async(
        async move {
          let opt_source = match module_specifier.scheme() {
            "data" => OptionSource::Mime(DataUrl::process(module_specifier.as_str()).map_err(|_| generic_error("URL has scheme \"data\" but is not a valid Data Url"))?.mime_type().to_string()),
            "file" | "http" | "https" => OptionSource::Path(PathBuf::from(module_specifier.path().to_string())),
            "sjs" | "blob" | _ => OptionSource::None
          };

          let mut mtsc_options = mtsc::Options {
            module: true,
            preprocess: false,
            transpile: false,
            filename: Some(module_specifier.clone().into()),
            macros,
            include_paths,
            ..Default::default()
          };

          mtsc::util::update_options(opt_source.clone(), &mut mtsc_options, &mtsc::util::all_options());

          if opt_source == OptionSource::None {
            mtsc_options.preprocess = true;
            mtsc_options.transpile = true;
          }

          let module_type = match &opt_source {
            OptionSource::Mime(mime_type) => match mime_type.as_str() {
              "application/json" => Some(ModuleType::Json),
              "application/wasm" => Some(ModuleType::Wasm),
              _ => None
            },
            OptionSource::Path(path) => path.extension().and_then(|ext| ext.to_str()).and_then(|ext| match ext {
              "json" => Some(ModuleType::Json),
              "wasm" => Some(ModuleType::Wasm),
              _ => None
            }),
            _ => None
          }.unwrap_or_else(|| match &requested_module_type {
            RequestedModuleType::Other(ty) => ModuleType::Other(ty.clone()),
            _ => ModuleType::JavaScript,
          });

          if module_type == ModuleType::Json && requested_module_type != RequestedModuleType::Json {
            return Err(generic_error("Attempted to load JSON module without specifying \"type\": \"json\" attribute in the import statement"));
          }

          let code = file_fetcher.fetch(&module_specifier,PermissionsContainer::allow_all()).await.map_err(|x| generic_error(format!("{}: {}",module_specifier,x)))?.source.clone();
          
          let code = if module_type == ModuleType::JavaScript {
            ModuleSourceCode::String(mtsc::compile(std::str::from_utf8(&code)?,&mtsc_options).ok_or_else(|| generic_error("Failed to compile script"))?.into())
          } else {
            ModuleSourceCode::Bytes(code.into())
          };

          return Ok(ModuleSource::new(
            module_type,
            code,
            &module_specifier,
            None // TODO implement source code cache?
          ))
        }.boxed_local()
      )
    }
  }