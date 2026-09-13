// Application state
#[derive(Clone)]
pub struct ScraperState {
    pub http_client: reqwest::Client,
    pub dns_pins: crate::core::dns::PinnedDns,
}
