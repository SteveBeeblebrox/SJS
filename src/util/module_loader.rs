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

pub struct SJSModuleLoader {
  // map: HashMap<ModuleSpecifier, ModuleCodeString>,
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
      let fut = async move {
        let path = module_specifier.to_file_path().map_err(|_| {
          generic_error(format!(
            "Provided module specifier \"{module_specifier}\" is not a file URL."
          ))
        })?;
        let module_type = if let Some(extension) = path.extension() {
          let ext = extension.to_string_lossy().to_lowercase();
          // We only return JSON modules if extension was actually `.json`.
          // In other cases we defer to actual requested module type, so runtime
          // can decide what to do with it.
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
  
        // If we loaded a JSON file, but the "requested_module_type" (that is computed from
        // import attributes) is not JSON we need to fail.
        if module_type == ModuleType::Json
          && requested_module_type != RequestedModuleType::Json
        {
          return Err(generic_error("Attempted to load JSON module without specifying \"type\": \"json\" attribute in the import statement."));
        }
  
        let code = std::fs::read(path).with_context(|| {
          format!("Failed to load {}", module_specifier.as_str())
        })?;
        let module = ModuleSource::new(
          module_type,
          ModuleSourceCode::Bytes(code.into_boxed_slice().into()),
          &module_specifier,
        );
        Ok(module)
      }
      .boxed_local();
  
      ModuleLoadResponse::Async(fut)
    }
  }