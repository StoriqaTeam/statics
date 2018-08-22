//! Config module contains the top-level config for the app.

use config_crate::{Config as RawConfig, ConfigError, Environment, File};
use std::env;
use stq_logging;

/// Global app config
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Server,
    pub client: Client,
    pub s3: S3,
    pub jwt: JWT,
    /// GrayLog settings
    pub graylog: Option<stq_logging::GrayLogConfig>,
}

fn default_acao() -> String {
    "*".into()
}

/// Common server settings
#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_acao")]
    pub acao: String,
}

/// AWS S3 credentials
#[derive(Debug, Deserialize, Clone)]
pub struct S3 {
    pub key: String,
    pub secret: String,
    pub region: String,
    pub bucket: String,
}

/// JWT data
#[derive(Debug, Deserialize, Clone)]
pub struct JWT {
    pub public_key_path: String,
    pub leeway: i64,
}

/// Http client settings
#[derive(Debug, Deserialize, Clone)]
pub struct Client {
    pub http_client_retries: usize,
    pub http_client_buffer_size: usize,
    pub dns_worker_thread_count: usize,
    pub timeout_duration_ms: u64,
}

/// Creates new app config struct. The order is take `base.toml`, then override with
/// `development/test/production.toml`, then override with `STQ_STATICS_` env variables.
/// #Examples
/// ```
/// use statics_lib::Config;
///
/// let config = Config::new();
/// ```
impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = RawConfig::new();
        s.merge(File::with_name("config/base"))?;

        // Note that this file is _optional_
        let env = env::var("RUN_MODE").unwrap_or("development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;

        // Add in settings from the environment (with a prefix of STQ_STATICS)
        s.merge(Environment::with_prefix("STQ_STATICS"))?;

        s.try_into()
    }
}
