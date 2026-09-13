use aide::{
    OperationOutput,
    generate::GenContext,
    openapi::{MediaType, Operation, Response as ApiResponse, StatusCode},
};
use axum::{
    Json,
    http::StatusCode as HttpStatus,
    response::{IntoResponse, Response},
};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Serialize;

// HTTP Problem

/// Validation constraint violation details
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Violation {
    /// The field for which the validation failed
    pub field: String,
    /// Part of the http request where the validation error occurred such as query, path, header, form, body
    #[serde(rename = "in")]
    pub location: String,
    /// Description of the validation error
    pub message: String,
}

/// HTTP Problem Response according to RFC9457 and RFC7807
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct HttpProblem {
    /// A optional URI reference that identifies the problem type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_uri: Option<String>,
    /// A optional, short, human-readable summary of the problem type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The HTTP status code for this occurrence of the problem
    pub status: i32,
    /// A optional human-readable explanation specific to this occurrence of the problem
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

// HttpValidationProblem
/// HTTP Validation Problem Response according to RFC9457 and RFC7807
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct HttpValidationProblem {
    /// A optional URI reference that identifies the problem type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_uri: Option<String>,
    /// A optional, short, human-readable summary of the problem type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The HTTP status code for this occurrence of the problem
    pub status: i32,
    /// A optional human-readable explanation specific to this occurrence of the problem
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// List of validation constraint violations that occurred
    pub violations: Vec<Violation>,
}

#[derive(Debug)]
pub struct Problem {
    status: HttpStatus,
    title: &'static str,
    detail: String,
    violations: Option<Vec<Violation>>,
}

impl Problem {
    fn new(
        status: HttpStatus,
        title: &'static str,
        detail: impl Into<String>,
        violations: Option<Vec<Violation>>,
    ) -> Self {
        Self {
            status,
            title,
            detail: detail.into(),
            violations,
        }
    }

    pub fn bad_json(detail: impl Into<String>) -> Self {
        Self::new(HttpStatus::BAD_REQUEST, "Invalid JSON", detail, None)
    }

    pub fn validation(violations: Vec<Violation>) -> Self {
        Self::new(
            HttpStatus::UNPROCESSABLE_ENTITY,
            "Unprocessable Entity",
            "Validation failed",
            Some(violations),
        )
    }

    pub fn unprocessable(detail: impl Into<String>) -> Self {
        Self::validation(vec![Violation {
            field: "url".into(),
            location: "body".into(),
            message: detail.into(),
        }])
    }

    pub fn unsupported_media() -> Self {
        Self::new(
            HttpStatus::UNSUPPORTED_MEDIA_TYPE,
            "Unsupported Media Type",
            "Only text/html can be scrapped",
            None,
        )
    }

    pub fn payload_too_large() -> Self {
        Self::new(
            HttpStatus::PAYLOAD_TOO_LARGE,
            "Payload Too Large",
            "Response exceeds size limit",
            None,
        )
    }

    pub fn bad_gateway(detail: impl Into<String>) -> Self {
        Self::new(HttpStatus::BAD_GATEWAY, "Bad Gateway", detail, None)
    }

    pub fn gateway_timeout() -> Self {
        Self::new(
            HttpStatus::GATEWAY_TIMEOUT,
            "Gateway Timeout",
            "Upstream fetch timed out",
            None,
        )
    }

    pub fn unexpected() -> Self {
        Self::new(
            HttpStatus::INTERNAL_SERVER_ERROR,
            "Unexpected Error",
            "Unexpected internal error",
            None,
        )
    }
}

// Axum
impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let status = self.status;
        if let Some(violations) = self.violations {
            let body = HttpValidationProblem {
                type_uri: None,
                title: Some(self.title.into()),
                status: status.as_u16() as i32,
                detail: Some(self.detail),
                violations,
            };

            (status, problem_json(body)).into_response()
        } else {
            let body = HttpProblem {
                type_uri: None,
                title: Some(self.title.into()),
                status: status.as_u16() as i32,
                detail: Some(self.detail),
            };

            (status, problem_json(body)).into_response()
        }
    }
}

fn problem_json<T: Serialize>(body: T) -> Response {
    let mut response = Json(body).into_response();

    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/problem+json"),
    );
    response
}

// Aide
fn problem_response<T: JsonSchema>(context: &mut GenContext, description: &str) -> ApiResponse {
    let schema = context.schema.subschema_for::<T>();

    ApiResponse {
        description: description.into(),
        content: IndexMap::from_iter([(
            "application/problem+json".into(),
            MediaType {
                schema: Some(aide::openapi::SchemaObject {
                    json_schema: schema,
                    example: None,
                    external_docs: None,
                }),
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

impl OperationOutput for Problem {
    type Inner = Self;

    fn inferred_responses(
        context: &mut GenContext,
        _operation: &mut Operation,
    ) -> Vec<(Option<StatusCode>, ApiResponse)> {
        vec![
            (
                Some(StatusCode::Code(400)),
                problem_response::<HttpProblem>(context, "Bad request."),
            ),
            (
                Some(StatusCode::Code(422)),
                problem_response::<HttpValidationProblem>(context, "Unprocessable Entity."),
            ),
            (
                Some(StatusCode::Code(413)),
                problem_response::<HttpProblem>(context, "Payload Too Large."),
            ),
            (
                Some(StatusCode::Code(415)),
                problem_response::<HttpProblem>(context, "Unsupported Media Type."),
            ),
            (
                Some(StatusCode::Code(502)),
                problem_response::<HttpProblem>(context, "Bad Gateway."),
            ),
            (
                Some(StatusCode::Code(504)),
                problem_response::<HttpProblem>(context, "Gateway Timeout."),
            ),
            (
                Some(StatusCode::Code(500)),
                problem_response::<HttpProblem>(context, "Unexpected error."),
            ),
        ]
    }
}
