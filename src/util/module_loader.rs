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

use std::path::Path;
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
        Some(import_map) => Ok(import_map.resolve(specifier, &util::resolve_maybe_url(referrer)?)?),
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
          let maybe_ext = Path::new(&module_specifier.path()).extension().map(|ext| ext.to_string_lossy().to_lowercase());

          let mut mtsc_options = mtsc::Options {
            module: true,
            preprocess: true,
            transpile: false,
            filename: Some(module_specifier.clone().into()),
            macros,
            include_paths,
            ..Default::default()
          };

          let module_type = if let Some(ext) = maybe_ext {
            mtsc::util::update_options_by_ext(ext.clone(), &mut mtsc_options, &mtsc::util::all_ext_options());

            match ext.as_str() {
              "json" => ModuleType::Json,
              _ => match &requested_module_type {
                RequestedModuleType::Other(ty) => ModuleType::Other(ty.clone()),
                _ => ModuleType::JavaScript,
              }
            }
          } else {
            ModuleType::JavaScript
          };

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