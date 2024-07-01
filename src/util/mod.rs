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

mod path_util;
pub use path_util::ToAbsolutePath;

mod hash;
pub use hash::hash;

mod cert;
#[allow(unused_imports)]
pub use cert::{CaData,BasicRootCertStoreProvider};

mod url;
#[allow(unused_imports)]
pub use url::{Url,resolve_maybe_url};

pub fn get_user_agent() -> &'static str {
    concat!("sjs/", env!("CARGO_PKG_VERSION"))
}
