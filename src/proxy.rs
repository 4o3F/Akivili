use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use log::info;
use serde::{Deserialize, Serialize};

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
        Some(self.last_used.cmp(&other.last_used))
        // if self.proxy_ip == other.proxy_ip {
        //     if self.proxy_port == other.proxy_port {
        //         Some(self.proxy_type.cmp(&other.proxy_type))
        //     } else {
        //         Some(self.proxy_port.cmp(&other.proxy_port))
        //     }
        // } else {
        //     Some(self.proxy_ip.cmp(&other.proxy_ip))
        // }
    }
}

impl Ord for Proxy {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}