mod auth_tokens;

mod cache_settings;
pub use cache_settings::CacheSetting;

mod cache_env;
pub use cache_env::SJSCacheEnv;

mod file_fetcher;
pub use file_fetcher::FileFetcher;
pub use file_fetcher::File;

mod http_util;
pub use http_util::HttpClient;

mod module_loader;
pub use module_loader::SJSModuleLoader;

pub fn get_user_agent() -> String {
    format!("sjs/{}", env!("CARGO_PKG_VERSION"))
}
