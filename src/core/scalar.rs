use axum::Router;
use scalar_api_reference::axum::routes;
use serde_json::json;

// Router
pub fn scalar_router() -> Router {
    let configuration = json!({
        "url": "/openapi.json"
    });

    let (scalar_route, asset_route) = routes("/docs", &configuration);

    Router::new().merge(scalar_route).merge(asset_route)
}
