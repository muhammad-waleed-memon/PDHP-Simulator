//! FHIR Interoperability Module (Phase 5) — Standards-compliant FHIR R4 adaptation layer.

pub mod mappers;
pub mod model;
pub mod routes;
pub mod service;

pub use routes::router;
