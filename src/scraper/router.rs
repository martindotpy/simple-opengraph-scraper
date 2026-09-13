use crate::{
    core::{extractor::ValidJson, problem::Problem, state::ScraperState},
    scraper::{
        dto::{OpengraphBody, OpengraphResponse},
        service::OpengraphScraperService,
        usecase::extract_opengraph,
    },
};
use aide::{
    axum::{ApiRouter, routing::post_with},
    transform::TransformOperation,
};
use axum::extract::State;

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
