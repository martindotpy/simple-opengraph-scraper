use aide::{
    OperationInput,
    generate::GenContext,
    openapi::{MediaType, Operation, RequestBody, Response as ApiResponse, StatusCode},
};
use axum::{
    Json,
    extract::{FromRequest, Request},
};
use garde::Validate;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use tracing::warn;

use crate::core::problem::{HttpProblem, HttpValidationProblem, Problem, Violation};

// Extractor
pub struct ValidJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
    T::Context: Default,
{
    type Rejection = Problem;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|error| {
            warn!(error = %error, "rejected malformed json");

            Problem::bad_json(error.to_string())
        })?;

        value.validate().map_err(|report| {
            warn!(error = %report, "rejected invalid body");

            let violations = report
                .into_inner()
                .into_iter()
                .map(|(path, error)| {
                    let raw_path = path.to_string();
                    Violation {
                        field: if raw_path.is_empty() {
                            "$".into()
                        } else {
                            raw_path
                        },
                        location: "body".into(),
                        message: error.message().into(),
                    }
                })
                .collect();

            Problem::validation(violations)
        })?;

        Ok(Self(value))
    }
}

// Aide
impl<T: JsonSchema> OperationInput for ValidJson<T> {
    fn operation_input(context: &mut GenContext, operation: &mut Operation) {
        let schema = context.schema.subschema_for::<T>();
        let resolved = context.resolve_schema(&schema);

        aide::operation::set_body(
            context,
            operation,
            RequestBody {
                description: resolved
                    .get("description")
                    .and_then(|description| description.as_str())
                    .map(String::from),
                content: IndexMap::from_iter([(
                    "application/json".into(),
                    MediaType {
                        schema: Some(aide::openapi::SchemaObject {
                            json_schema: schema,
                            example: None,
                            external_docs: None,
                        }),
                        ..Default::default()
                    },
                )]),
                required: true,
                extensions: IndexMap::default(),
            },
        );
    }

    fn inferred_early_responses(
        context: &mut GenContext,
        _operation: &mut Operation,
    ) -> Vec<(Option<StatusCode>, ApiResponse)> {
        vec![
            (
                Some(StatusCode::Code(400)),
                error_response::<HttpProblem>(context, "Bad request."),
            ),
            (
                Some(StatusCode::Code(422)),
                error_response::<HttpValidationProblem>(context, "Unprocessable Entity."),
            ),
        ]
    }
}

fn error_response<T: JsonSchema>(context: &mut GenContext, description: &str) -> ApiResponse {
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
