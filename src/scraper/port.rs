use async_trait::async_trait;

use crate::scraper::domain::{Opengraph, ScrapeError};

// Port
#[async_trait]
pub trait OpengraphScraperPort: Sync {
    async fn scrape_opengraph(&self, url: &str) -> Result<Opengraph, ScrapeError>;
}
