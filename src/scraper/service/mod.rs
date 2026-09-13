mod bytes;
mod entities;
mod fetcher;
mod guards;
mod meta;
mod scanner;

use async_trait::async_trait;
use derive_new::new;
use tracing::debug;
use crate::scraper::{
    domain::{Opengraph, OpengraphDescription, OpengraphImage, OpengraphTitle, ScrapeError},
    port::OpengraphScraperPort,
};
use fetcher::fetch_validated;
use guards::{ensure_html, ensure_size, ensure_success};
use scanner::OpengraphHtmlScanner;

#[derive(new)]
pub struct OpengraphScraperService {
    http_client: reqwest::Client,
    dns_pins: crate::core::dns::PinnedDns,
}

#[async_trait]
impl OpengraphScraperPort for OpengraphScraperService {
    async fn scrape_opengraph(&self, url: &str) -> Result<Opengraph, ScrapeError> {
        // Host for logs only: validation happens per hop inside fetch_validated
        let host = url::Url::parse(url)
            .ok()
            .and_then(|parsed| parsed.host_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string());

        let started_instant = std::time::Instant::now();
        let response = fetch_validated(&self.http_client, &self.dns_pins, url, &host).await?;

        ensure_success(&response, &host)?;
        ensure_size(&response, &host)?;
        ensure_html(&response, &host)?;

        let scanner = OpengraphHtmlScanner::collect(response, &host).await?;

        debug!(
            host = %host,
            elapsed_ms = started_instant.elapsed().as_millis(),
            bytes = scanner.total,
            title = ?scanner.title,
            description = ?scanner.description,
            image = ?scanner.image,
            "extracted opengraph data"
        );

        Ok(Opengraph::new(
            OpengraphTitle(scanner.title),
            OpengraphDescription(scanner.description),
            OpengraphImage(scanner.image),
        ))
    }
}
