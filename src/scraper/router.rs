use aide::{
    axum::{ApiRouter, routing::post_with},
    transform::TransformOperation,
};
use axum::extract::State;

use crate::{
    core::{extractor::ValidJson, problem::Problem},
    scraper::{
        domain::ScrapeError,
        dto::{OpengraphBody, OpengraphResponse},
        service::OpengraphScraperService,
        state::ScraperState,
        url::DeniedUrlError,
        usecase::extract_opengraph,
    },
};

// Router
pub fn opengraph_router(state: ScraperState) -> ApiRouter {
    ApiRouter::new()
        .nest(
            "/opengraph",
            ApiRouter::new().api_route("/", post_with(post_opengraph, post_opengraph_docs)),
        )
        .with_state(state)
}

// Post
fn post_opengraph_docs(operation: TransformOperation) -> TransformOperation {
    operation
        .summary("Get Opengraph data")
        .description("Retrieve OpenGraph data for a given URL")
        .tag("Opengraph")
}

async fn post_opengraph(
    State(state): State<ScraperState>,
    ValidJson(body): ValidJson<OpengraphBody>,
) -> Result<OpengraphResponse, Problem> {
    let scraper_service = OpengraphScraperService::new(state.http_client, state.dns_pins);
    let opengraph_data = extract_opengraph(&body.url, &scraper_service).await?;

    Ok(OpengraphResponse::new(
        opengraph_data.into(),
        "Opengraph data retrieved successfully".into(),
    ))
}

// Errors
impl From<DeniedUrlError> for Problem {
    fn from(error: DeniedUrlError) -> Self {
        match error {
            DeniedUrlError::InvalidUrl => Problem::unprocessable("url must be a valid URL"),
            DeniedUrlError::NotHttp => Problem::unprocessable("only http/https urls are allowed"),
            DeniedUrlError::Credentials => {
                Problem::unprocessable("url must not contain credentials")
            }
            DeniedUrlError::ForbiddenPort => Problem::unprocessable("url port is not allowed"),
            DeniedUrlError::ForbiddenHost => Problem::unprocessable("url host is not allowed"),
            DeniedUrlError::Unresolvable => {
                Problem::unprocessable("url host could not be resolved")
            }
        }
    }
}

impl From<ScrapeError> for Problem {
    fn from(error: ScrapeError) -> Self {
        match error {
            ScrapeError::Denied(denied_url_error) => denied_url_error.into(),
            ScrapeError::Upstream => Problem::bad_gateway("failed to fetch url"),
            ScrapeError::UnsupportedMedia => Problem::unsupported_media(),
            ScrapeError::TooLarge => Problem::payload_too_large(),
            ScrapeError::Timeout => Problem::gateway_timeout(),
            ScrapeError::Unexpected => Problem::unexpected(),
        }
    }
}
