use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use log::info;
use serde::Deserialize;
use tokio::sync::Semaphore;

use crate::checker;
use crate::provider::ProxyProvider;
use crate::proxy::{Proxy, ProxyType};
use crate::time::current_timestamp;

pub struct CheckerProxyProvider {
    last_fetched: u64,
}

#[derive(Deserialize, Debug)]
struct CheckerProxyArchive {
    date: String,
}

#[derive(Deserialize, Debug)]
struct CheckerProxy {
    #[serde(rename = "addr")]
    ip: String,
    #[serde(rename = "type")]
    proxy_type: i32,
    #[serde(rename = "addr_geo_iso")]
    country: String,
}

#[async_trait]
impl ProxyProvider for CheckerProxyProvider {
    const PROXY_IDENTIFIER: &'static str = "checkproxy.net";

    fn new() -> Self {
        CheckerProxyProvider { last_fetched: 0 }
    }
    fn get_last_fetch(&self) -> u64 {
        self.last_fetched
    }
    async fn fetch(&mut self) -> Result<Vec<Proxy>> {
        let client = reqwest::ClientBuilder::new()
            .no_proxy()
            .build()?;
        // Get the latest archive
        let request = client.get("https://checkerproxy.net/api/archive").build()?;
        let response = client.execute(request).await?;
        let mut datas: Vec<CheckerProxyArchive> = response.json::<Vec<CheckerProxyArchive>>().await?;
        datas.sort_by(|a, b| b.date.cmp(&a.date));
        let latest_archive = datas.get(0).unwrap();
        info!("Latest data: {}", latest_archive.date);
        // Get the latest proxy data
        let request = client.get("https://checkerproxy.net/api/archive/".to_owned() + latest_archive.date.as_str()).build()?;
        let response = client.execute(request).await?;
        let datas: Vec<CheckerProxy> = response.json::<Vec<CheckerProxy>>().await?;

        // Check proxy available
        let proxies: Arc<Mutex<Vec<Proxy>>> = Arc::new(Mutex::new(Vec::new()));
        let mut tasks = Vec::new();
        let semaphore = Arc::new(Semaphore::new(10));
        for data in datas {
            let (address, port) = data.ip.split_once(":").unwrap();
            let address = address.to_string();
            let port = port.parse::<i32>()?;
            let mut proxy: Proxy = Proxy {
                proxy_type: match data.proxy_type {
                    4 => ProxyType::SOCKS5,
                    1 => ProxyType::HTTP,
                    2 => ProxyType::HTTPS,
                    _ => {
                        panic!("Unknown proxy type in CheckerProxy provider")
                    }
                },
                proxy_ip: address,
                proxy_port: port,
                country: data.country,
                last_checked: 0,
                last_used: 0,
            };
            // TODO: Implement http proxy chain and remove this
            // if proxy.proxy_type == ProxyType::HTTP {
            //     continue;
            // }

            let proxies = Arc::clone(&proxies);
            let semaphore = Arc::clone(&semaphore);
            tasks.push(tokio::spawn(async move {
                let _ = semaphore.acquire().await.unwrap();
                let status = checker::check_proxy(&proxy).await;
                if status.as_ref().is_err() {
                    info!("Proxy {} unavailable", proxy.proxy_ip);
                    return;
                }
                proxy.last_checked = current_timestamp();
                proxy.last_used = current_timestamp();
                info!("Proxy {} available", proxy.proxy_ip);
                let mut proxies = proxies.lock().unwrap();
                proxies.push(proxy);
            }));
        }
        for task in tasks {
            task.await?;
        }
        let proxies = Arc::try_unwrap(proxies).unwrap();
        let proxies = proxies.into_inner().unwrap();
        self.last_fetched = current_timestamp();
        Ok(proxies)
    }
}