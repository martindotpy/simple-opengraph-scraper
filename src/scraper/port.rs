use crate::scraper::domain::{Opengraph, ScrapeError};
use async_trait::async_trait;

// Port
#[async_trait]
pub trait OpengraphScraperPort: Sync {
    async fn scrape_opengraph(&self, url: &str) -> Result<Opengraph, ScrapeError>;
}
