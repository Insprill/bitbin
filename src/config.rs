use anyhow::{bail, Result};
use bitbin::CopyNonDefaults;
use log::info;
use std::{fs, path::Path};

use serde::Deserialize;

const CONFIG_PATH: &str = "config.toml";

#[derive(Clone, Deserialize, Default, Debug, CopyNonDefaults)]
#[serde(default)]
pub struct Config {
    //#[serde_inline_default(ServerConfig::default())]
    pub http: HttpConfig,
    pub misc: MiscConfig,
    pub content: ContentConfig,
}

#[derive(Clone, Deserialize, Debug, PartialEq, CopyNonDefaults)]
#[serde(default)]
pub struct HttpConfig {
    /// Sets the address to listen on
    pub host: String,

    /// Sets the port to listen on. Will be overriden by the PORT env var if present
    pub port: u16,

    /// The amount of HTTP workers to use. 0 to equal physical CPU cores
    pub workers: usize,

    /// The Keep-Alive timeout, in seconds. Set to 0 to disable.
    pub keep_alive_timeout: f32,

    /// Whether TLS should be used
    pub tls: bool,

    /// The path to the KEY file. Required when using TLS.
    pub tls_key_file: Option<String>,

    /// The path to the CERT file. Required when using TLS.
    pub tls_cert_file: Option<String>,
}

#[derive(Clone, Copy, Deserialize, Debug, PartialEq, CopyNonDefaults)]
#[serde(default)]
pub struct MiscConfig {
    /// The length of generated keys in characters
    pub keylength: usize,
}

#[derive(Clone, Copy, Deserialize, Debug, PartialEq, CopyNonDefaults)]
#[serde(default)]
pub struct ContentConfig {
    /// Max content length in MB
    pub maxsize: usize,
}

impl Config {
    pub fn create() -> Result<Config> {
        let (mut http, mut misc, mut content) = Self::from_env("BYTEBIN");
        let (b_http, b_misc, b_content) = Self::from_env("BITBIN");

        http.copy_non_defaults(&b_http);
        misc.copy_non_defaults(&b_misc);
        content.copy_non_defaults(&b_content);

        let config_path = Path::new(CONFIG_PATH);
        if !config_path.exists() {
            info!("No config found at {}!", config_path.to_string_lossy());
            return Ok(Config {
                http,
                misc,
                content,
            });
        }

        let config_str = fs::read_to_string(CONFIG_PATH)?;
        let mut config: Config = match toml::from_str(&config_str) {
            Ok(cfg) => cfg,
            Err(err) => {
                bail!("Failed to read config file! {}", err);
            }
        };

        config.http.copy_non_defaults(&http);
        config.misc.copy_non_defaults(&misc);
        config.content.copy_non_defaults(&content);

        Ok(config)
    }

    fn from_env(prefix: &str) -> (HttpConfig, MiscConfig, ContentConfig) {
        let http = envy::prefixed(format!("{}_HTTP_", prefix))
            .from_env::<HttpConfig>()
            .unwrap_or_default();
        let misc = envy::prefixed(format!("{}_MISC_", prefix))
            .from_env::<MiscConfig>()
            .unwrap_or_default();
        let content = envy::prefixed(format!("{}_CONTENT_", prefix))
            .from_env::<ContentConfig>()
            .unwrap_or_default();
        (http, misc, content)
    }
}

// Keep the defaults in sync with config.toml and .env.example!

impl Default for HttpConfig {
    fn default() -> Self {
        HttpConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 0,
            keep_alive_timeout: 15.0,
            tls: false,
            tls_key_file: Option::None,
            tls_cert_file: Option::None,
        }
    }
}

impl Default for MiscConfig {
    fn default() -> Self {
        MiscConfig { keylength: 7 }
    }
}

impl Default for ContentConfig {
    fn default() -> Self {
        ContentConfig { maxsize: 10 }
    }
}
