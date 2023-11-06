use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use log::info;
use serde::Deserialize;
use crate::checker;
use crate::provider::ProxyProvider;
use crate::proxy::{Proxy, ProxyType};
use crate::time::current_timestamp;
use anyhow::Result;
use tokio::sync::Semaphore;

pub struct DocIPProvider {
    last_fetched: u64,
}

#[derive(Deserialize, Debug)]
struct DocIPJSON {
    data: Vec<DocIPProxy>,
}

#[derive(Deserialize, Debug)]
struct DocIPProxy {
    ip: String,
    addr: String,
    proxy_type: String,
}

#[async_trait]
impl ProxyProvider for DocIPProvider {
    const PROXY_IDENTIFIER: &'static str = "docip.net";

    fn new() -> Self {
        DocIPProvider {
            last_fetched: 0
        }
    }

    fn get_last_fetch(&self) -> u64 {
        self.last_fetched
    }

    async fn fetch(&mut self) -> Result<Vec<Proxy>> {
        let client = reqwest::ClientBuilder::new()
            .no_proxy()
            .build()?;
        let request = client.get("https://www.docip.net/data/free.json").build()?;
        let response = client.execute(request).await?;
        let proxies: Arc<Mutex<Vec<Proxy>>> = Arc::new(Mutex::new(Vec::new()));
        let data = response.json::<DocIPJSON>().await?;

        let mut tasks = Vec::new();
        let semaphore = Arc::new(Semaphore::new(10));
        for docip_proxy in data.data {
            let (address, port) = docip_proxy.ip.split_once(":").unwrap();
            let address = address.to_string();
            let port = port.parse::<i32>()?;
            let mut proxy: Proxy = Proxy {
                proxy_type: match docip_proxy.proxy_type.as_str() {
                    "2" => ProxyType::HTTP,
                    "1" => ProxyType::HTTPS,
                    _ => {
                        panic!("Unknown proxy type in DocIP provider")
                    }
                },
                proxy_ip: address,
                proxy_port: port,
                country: docip_proxy.addr,
                last_checked: 0,
                last_used: 0,
            };
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
        info!("Available proxies: {:?}", proxies);
        Ok(proxies)
    }
}