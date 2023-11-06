use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub check_timeout: u64,
    pub check_interval: u64,
    pub update_interval: u64,
    pub provider_docip_enabled: bool,
    pub provider_checkerproxy_enabled: bool,
}

impl Config {
    pub fn default() -> Config {
        Config {
            check_timeout: 10,
            check_interval: 300,
            update_interval: 6000,
            provider_docip_enabled: true,
            provider_checkerproxy_enabled: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::default()
    }
}