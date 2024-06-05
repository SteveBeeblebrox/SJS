use deno_runtime::deno_core;
use deno_core::{
  ModuleSpecifier, ModuleSource, ModuleType, RequestedModuleType, ResolutionKind,
  resolve_import, ModuleSourceCode, ModuleLoader, ModuleLoadResponse
};
use deno_core::anyhow::Error;
use deno_core::error::generic_error;
use deno_runtime::permissions::PermissionsContainer;
use deno_core::futures::FutureExt;

use std::path::Path;
use std::sync::Arc;


use or_panic::OrPanic;

use crate::util::FileFetcher;

pub struct SJSModuleLoader {
  pub file_fetcher: Arc<FileFetcher>
}

impl ModuleLoader for SJSModuleLoader {
    fn resolve(
      &self,
      specifier: &str,
      referrer: &str,
      _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, Error> {
      Ok(resolve_import(specifier, referrer)?)
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
      return ModuleLoadResponse::Async(
        async move {
          let module_type = if let Some(extension) = Path::new(&module_specifier.path()).extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            if ext == "json" {
              ModuleType::Json
            } else {
              match &requested_module_type {
                RequestedModuleType::Other(ty) => ModuleType::Other(ty.clone()),
                _ => ModuleType::JavaScript,
              }
            }
          } else {
            ModuleType::JavaScript
          };

          if module_type == ModuleType::Json && requested_module_type != RequestedModuleType::Json {
            return Err(generic_error("Attempted to load JSON module without specifying \"type\": \"json\" attribute in the import statement."));
          }

          let code = file_fetcher.fetch(&module_specifier,PermissionsContainer::allow_all()).await.map_err(|x| format!("{}: {}",module_specifier,x)).or_panic().source.clone();
          // TODO mtsc transform code here
          Ok(ModuleSource::new(
            module_type,
            ModuleSourceCode::Bytes(code.into()),
            &module_specifier,
            None // TODO implement source code cache?
          ))
        }.boxed_local()
      )
    }
  }