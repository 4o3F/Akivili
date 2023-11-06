pub mod docip;
pub mod checkerproxy;

use async_trait::async_trait;
use crate::proxy::Proxy;
use anyhow::Result;

#[async_trait]
pub trait ProxyProvider {
    const PROXY_IDENTIFIER: &'static str;
    fn new() -> Self;
    fn get_last_fetch(&self) -> u64;
    async fn fetch(&mut self) -> Result<Vec<Proxy>>;
}