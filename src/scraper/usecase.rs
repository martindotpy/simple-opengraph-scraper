use crate::scraper::{
    domain::{Opengraph, ScrapeError},
    port::OpengraphScraperPort,
};

// Use case
pub async fn extract_opengraph(
    url: &str,
    opengraph_scraper_port: &dyn OpengraphScraperPort,
) -> Result<Opengraph, ScrapeError> {
    opengraph_scraper_port.scrape_opengraph(url).await
}
