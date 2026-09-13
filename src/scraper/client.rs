use reqwest::{Client, redirect::Policy};
use std::sync::Arc;
use std::time::Duration;

use crate::core::dns::PinnedDns;

// Client
// Redirects are followed manually so every hop is SSRF-checked and DNS-pinned
// User-Agent is set per request with Meta crawler rotation (see fetcher)
pub fn build_http_client(dns: PinnedDns) -> Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(Duration::from_secs(4))
        .connect_timeout(Duration::from_secs(2))
        .pool_max_idle_per_host(20)
        .redirect(Policy::none())
        .dns_resolver(Arc::new(dns))
        .build()
}
