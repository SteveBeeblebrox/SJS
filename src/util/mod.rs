mod auth_tokens;

mod cache_settings;

mod file_fetcher;
pub use file_fetcher::FileFetcher;

mod http_util;

mod module_loader;
pub use module_loader::SJSModuleLoader;

pub fn get_user_agent() -> String {
    format!("sjs/{}", env!("CARGO_PKG_VERSION"))
}