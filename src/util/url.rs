pub use deno_runtime::deno_core::url::Url;
use deno_runtime::deno_core::error::AnyError;

use std::path::Path;

use super::ToAbsolutePath as _;

/// Converts a `&str` that could represent a file path or remote resource into a `Url`
pub fn resolve_maybe_url<S: AsRef<str>>(s: S) -> Result<Url, AnyError> {
    Url::parse(s.as_ref()).or_else(|_|
        Path::new(s.as_ref()).absolute().map(|x| Url::from_file_path(x.as_path()).unwrap())
        .map_err(|_x| AnyError::msg(format!("{}: {}",s.as_ref(),"Invalid file or URL")))
    )
}