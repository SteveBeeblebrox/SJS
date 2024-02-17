use deno_core::ModuleSpecifier;
use deno_core::ModuleCodeString;
use deno_core::ModuleSource;
use deno_core::ModuleType;
use deno_core::RequestedModuleType;
use deno_core::ResolutionKind;
use deno_core::resolve_import;
use deno_core::error::generic_error;
use deno_core::ModuleSourceCode;
use deno_core::anyhow::{Error,Context};
use deno_core::ModuleLoader;
use deno_core::ModuleLoadResponse;
use deno_core::futures::FutureExt;
use deno_runtime::permissions::PermissionsContainer;

use std::path::Path;

use crate::util::FileFetcher;

pub struct SJSModuleLoader {
  pub file_fetcher: FileFetcher
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

          if module_type == ModuleType::Json && requested_module_type != RequestedModuleType::Json
          {
            return Err(generic_error("Attempted to load JSON module without specifying \"type\": \"json\" attribute in the import statement."));
          }

          let code = file_fetcher.fetch(&module_specifier,PermissionsContainer::allow_all()).await.unwrap().source.clone();
          // TODO mtsc transform code here
          Ok(ModuleSource::new(
            module_type,
            ModuleSourceCode::Bytes(code.into()),
            &module_specifier,
          ))
        }.boxed_local()
      )
    }
  }