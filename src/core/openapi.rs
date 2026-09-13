use aide::transform::TransformOpenApi;

// General doc
pub fn openapi_docs(openapi: TransformOpenApi) -> TransformOpenApi {
    openapi
        .title("Simple OpenGraph Scraper")
        .description("A simple OpenGraph metadata scraper")
}
