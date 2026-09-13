use aide::{
    OperationOutput,
    generate::GenContext,
    openapi::{Operation, Response as ApiResponse, StatusCode},
};
use axum::{
    Json,
    http::StatusCode as HttpStatusCode,
    response::{IntoResponse, Response},
};
use schemars::JsonSchema;
use serde::Serialize;

// Response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct DataResponse<T> {
    pub data: T,
    pub detail: String,
    #[serde(skip)]
    pub status_code: HttpStatusCode,
}

impl<T> DataResponse<T> {
    pub fn new(data: T, detail: String) -> Self {
        Self::with_status_code(HttpStatusCode::OK, data, detail)
    }

    pub fn with_status_code(status_code: HttpStatusCode, data: T, detail: String) -> Self {
        Self {
            data,
            detail,
            status_code,
        }
    }
}

// Axum
impl<T> IntoResponse for DataResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let status_code = self.status_code;

        (status_code, Json(self)).into_response()
    }
}

// Aide
impl<T> OperationOutput for DataResponse<T>
where
    T: Serialize + schemars::JsonSchema,
{
    type Inner = Self;

    fn operation_response(
        context: &mut GenContext,
        operation: &mut Operation,
    ) -> Option<ApiResponse> {
        let mut response = Json::<Self>::operation_response(context, operation)?;

        response.description = "Opengraph data retrieved successfully".into();

        Some(response)
    }

    fn inferred_responses(
        context: &mut GenContext,
        operation: &mut Operation,
    ) -> Vec<(Option<StatusCode>, ApiResponse)> {
        let Some(response) = Self::operation_response(context, operation) else {
            return Vec::new();
        };

        vec![(Some(StatusCode::Code(200)), response)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_ok_status_code() {
        let response = DataResponse::new("data", "ok".to_string()).into_response();

        assert_eq!(response.status(), HttpStatusCode::OK);
    }

    #[test]
    fn honors_custom_status_code() {
        let response =
            DataResponse::with_status_code(HttpStatusCode::CREATED, "data", "created".to_string())
                .into_response();

        assert_eq!(response.status(), HttpStatusCode::CREATED);
    }

    #[test]
    fn serializes_body_without_status_code() {
        let body = serde_json::to_value(DataResponse::with_status_code(
            HttpStatusCode::CREATED,
            "data",
            "created".to_string(),
        ))
        .unwrap();

        assert_eq!(
            body,
            serde_json::json!({ "data": "data", "detail": "created" })
        );
    }
}
