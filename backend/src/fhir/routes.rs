//! FHIR Routes — REST API endpoints for FHIR R4 resources.

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::jwt::AuthenticatedUser;
use crate::error::AppError;
use crate::fhir::model::FhirOperationOutcome;
use crate::fhir::service::*;
use crate::state::AppState;

pub const FHIR_JSON_MEDIA_TYPE: &str = "application/fhir+json";

pub struct FhirResponse<T>(pub T);

impl<T: serde::Serialize> IntoResponse for FhirResponse<T> {
    fn into_response(self) -> Response {
        let json_body = match serde_json::to_string(&self.0) {
            Ok(j) => j,
            Err(e) => {
                let outcome = FhirOperationOutcome::error("exception", &e.to_string());
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, FHIR_JSON_MEDIA_TYPE)],
                    serde_json::to_string(&outcome).unwrap(),
                )
                    .into_response();
            }
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(FHIR_JSON_MEDIA_TYPE),
        );

        (StatusCode::OK, headers, json_body).into_response()
    }
}

pub fn fhir_error_response(err: AppError) -> Response {
    let (status, code, msg) = match &err {
        AppError::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            "login",
            "Authentication required".to_string(),
        ),
        AppError::InvalidCredentials => (
            StatusCode::UNAUTHORIZED,
            "login",
            "Invalid email or password".to_string(),
        ),
        AppError::TokenInvalid => (
            StatusCode::UNAUTHORIZED,
            "login",
            "Invalid or expired token".to_string(),
        ),
        AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg.clone()),
        AppError::ConsentRequired => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "Active consent grant required for this patient record".to_string(),
        ),
        AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "not-found", msg.clone()),
        AppError::Validation(msg) | AppError::BadRequest(msg) => {
            (StatusCode::BAD_REQUEST, "invalid", msg.clone())
        }
        AppError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone()),
        AppError::RateLimited => (
            StatusCode::TOO_MANY_REQUESTS,
            "throttled",
            "Rate limit exceeded".to_string(),
        ),
        AppError::Database(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
        AppError::Internal(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    };

    let outcome = FhirOperationOutcome::error(code, &msg);
    let json_body = serde_json::to_string(&outcome).unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(FHIR_JSON_MEDIA_TYPE),
    );

    (status, headers, json_body).into_response()
}

#[derive(Debug, Deserialize)]
pub struct PatientSearchParams {
    pub identifier: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatientResourceParams {
    pub patient: Option<Uuid>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/Patient/:id", get(get_patient))
        .route("/Patient", get(search_patient))
        .route("/Patient/:id/$everything", get(export_bundle))
        .route("/Practitioner/:id", get(get_practitioner))
        .route("/PractitionerRole/:id", get(get_practitioner_role_handler))
        .route("/Organization/:id", get(get_organization))
        .route("/Location/:id", get(get_location))
        .route("/Medication/:id", get(get_medication_handler))
        .route(
            "/MedicationStatement",
            get(search_medication_statements_handler),
        )
        .route("/Condition/:id", get(get_condition))
        .route("/Condition", get(search_conditions))
        .route("/AllergyIntolerance/:id", get(get_allergy))
        .route("/AllergyIntolerance", get(search_allergies))
        .route("/Encounter/:id", get(get_encounter))
        .route("/Encounter", get(search_encounters))
}

async fn get_patient(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id_str): Path<String>,
) -> Response {
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return fhir_error_response(AppError::BadRequest(format!("Invalid FHIR resource UUID: '{id_str}'"))),
    };
    match get_fhir_patient(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn search_patient(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<PatientSearchParams>,
) -> Response {
    if let Some(dhid) = query.identifier {
        match get_fhir_patient_by_dhid(&state, &user, &dhid).await {
            Ok(res) => FhirResponse(res).into_response(),
            Err(err) => fhir_error_response(err),
        }
    } else {
        fhir_error_response(AppError::BadRequest(
            "Search parameter 'identifier' (Digital Health ID) is required".to_string(),
        ))
    }
}

async fn export_bundle(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id_str): Path<String>,
) -> Response {
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return fhir_error_response(AppError::BadRequest(format!("Invalid FHIR resource UUID: '{id_str}'"))),
    };
    match export_patient_fhir_bundle(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn get_practitioner(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match get_fhir_practitioner(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn get_practitioner_role_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match get_fhir_practitioner_role(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn get_organization(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match get_fhir_organization(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn get_location(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match get_fhir_location(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn get_medication_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match get_fhir_medication(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn search_medication_statements_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<PatientResourceParams>,
) -> Response {
    let patient_id = match query.patient {
        Some(id) => id,
        None => {
            return fhir_error_response(AppError::BadRequest(
                "Search parameter 'patient' is required".to_string(),
            ))
        }
    };

    match search_fhir_medication_statements(&state, &user, patient_id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn get_condition(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match get_fhir_condition(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn search_conditions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<PatientResourceParams>,
) -> Response {
    let patient_id = match query.patient {
        Some(id) => id,
        None => {
            return fhir_error_response(AppError::BadRequest(
                "Search parameter 'patient' is required".to_string(),
            ))
        }
    };

    match search_fhir_conditions(&state, &user, patient_id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn get_allergy(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match get_fhir_allergy(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn search_allergies(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<PatientResourceParams>,
) -> Response {
    let patient_id = match query.patient {
        Some(id) => id,
        None => {
            return fhir_error_response(AppError::BadRequest(
                "Search parameter 'patient' is required".to_string(),
            ))
        }
    };

    match search_fhir_allergies(&state, &user, patient_id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn get_encounter(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match get_fhir_encounter(&state, &user, id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}

async fn search_encounters(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<PatientResourceParams>,
) -> Response {
    let patient_id = match query.patient {
        Some(id) => id,
        None => {
            return fhir_error_response(AppError::BadRequest(
                "Search parameter 'patient' is required".to_string(),
            ))
        }
    };

    match search_fhir_encounters(&state, &user, patient_id).await {
        Ok(res) => FhirResponse(res).into_response(),
        Err(err) => fhir_error_response(err),
    }
}
