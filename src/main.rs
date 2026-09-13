mod core;
mod scraper;

use std::time::Duration;

use aide::{axum::ApiRouter, openapi::OpenApi};
use axum::{Extension, Json, extract::DefaultBodyLimit, http::StatusCode, routing::get};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::{
    catch_panic::CatchPanicLayer, compression::CompressionLayer, timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::info;

use crate::core::state::ScraperState;

#[tokio::main]
async fn main() {
    // Tracing
    core::tracing::init_tracing();

    // State
    let dns_pins = core::dns::PinnedDns::default();
    let http_client = scraper::client::build_http_client(dns_pins.clone()).unwrap_or_else(|error| {
        tracing::error!(?error, "failed to build http client");

        std::process::exit(1);
    });
    let state = ScraperState {
        http_client,
        dns_pins,
    };

    // Router
    let mut openapi = OpenApi::default();

    let app = ApiRouter::new()
        // Api endpoints
        .nest_api_service("/api", scraper::router::opengraph_router(state.clone()))
        // Openapi
        .finish_api_with(&mut openapi, crate::core::openapi::openapi_docs)
        // State
        .with_state(state)
        .route("/openapi.json", get(get_openapi))
        // Scalar
        .merge(crate::core::scalar::scalar_router())
        // Middlewares
        .layer(Extension(openapi))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(CompressionLayer::new())
        .layer(ConcurrencyLimitLayer::new(126))
        .layer(DefaultBodyLimit::max(1024 * 512));

    // Listen
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    info!("Server listening on http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn get_openapi(Extension(openapi): Extension<OpenApi>) -> Json<OpenApi> {
    Json(openapi)
}
