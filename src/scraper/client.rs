use std::{sync::Arc, time::Duration};

use reqwest::{Client, redirect::Policy};

use crate::scraper::dns::PinnedDns;

// Client
pub fn build_http_client(dns: PinnedDns) -> Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(Duration::from_secs(4))
        .connect_timeout(Duration::from_secs(2))
        .pool_max_idle_per_host(20)
        .redirect(Policy::none())
        .dns_resolver(Arc::new(dns))
        .build()
}
