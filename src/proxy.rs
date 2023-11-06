use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::Path;

use anyhow::Result;
use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::PROXY_POOL;
use crate::time::current_timestamp;

#[derive(Deserialize, Serialize, Debug, Hash, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum ProxyType {
    HTTP,
    HTTPS,
    SOCKS5,
    SOCKS4,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Proxy {
    pub proxy_type: ProxyType,
    pub proxy_ip: String,
    pub proxy_port: i32,
    pub country: String,
    pub last_checked: u64,
    pub last_used: u64,
}

impl Hash for Proxy {
    fn hash<H: Hasher>(&self, state: &mut H) {
        info!("Hasher called");
        self.proxy_type.hash(state);
        self.proxy_ip.hash(state);
        self.proxy_port.hash(state);
        self.country.hash(state);
    }
}

impl PartialEq for Proxy {
    fn eq(&self, other: &Self) -> bool {
        self.proxy_type == other.proxy_type &&
            self.proxy_ip == other.proxy_ip &&
            self.proxy_port == other.proxy_port
    }
}

impl Eq for Proxy {}

impl PartialOrd for Proxy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.proxy_ip == other.proxy_ip && self.proxy_port == other.proxy_port && self.proxy_type == other.proxy_type {
            return Some(Ordering::Equal);
        }
        Some(self.last_used.cmp(&other.last_used))
    }
}

impl Ord for Proxy {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub fn init_proxy_pool() {
    let proxy_pool_file = Path::new("pool.json");
    if !proxy_pool_file.exists() {
        match save_proxy_pool() {
            Ok(_) => {
                info!("Successfully create proxy pool file");
            }
            Err(error) => {
                error!("Create proxy pool file failed, {}", error);
            }
        }
    } else {
        match load_proxy_pool() {
            Ok(proxy_pool) => {
                {
                    let mut global_proxy_pool = PROXY_POOL.lock().unwrap();
                    *global_proxy_pool = proxy_pool;
                    info!("Current proxy count: {}", global_proxy_pool.len());
                }
                info!("Successfully load proxy pool file, now checking...");
            }
            Err(error) => {
                error!("Load proxy pool file failed, {}", error);
            }
        }
    }
}

pub fn save_proxy_pool() -> Result<()> {
    let mut file = File::create("pool.json")?;
    let json = serde_json::to_string(PROXY_POOL.lock().unwrap().deref())?;
    match file.write_all(json.as_bytes()).map_err(anyhow::Error::from) {
        Ok(_) => {
            info!("Saved proxy pool {}", current_timestamp());
            Ok(())
        }
        Err(error) => {
            error!("Error saving proxy pool {} {}",current_timestamp(), error);
            Err(error)
        }
    }
}

pub fn load_proxy_pool() -> Result<BTreeSet<Proxy>> {
    let mut file = File::open("pool.json")?;
    let mut json: String = String::new();
    file.read_to_string(&mut json)?;
    let proxy_pool: BTreeSet<Proxy> = serde_json::from_str(json.as_str())?;
    Ok(proxy_pool)
}