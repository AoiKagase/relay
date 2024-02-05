use activitystreams::checked::CheckError;
use actix_web::{
    error::{BlockingError, ResponseError},
    http::StatusCode,
    HttpResponse,
};
use background_jobs::BoxError;
use color_eyre::eyre::Error as Report;
use http_signature_normalization_reqwest::SignError;
use std::{convert::Infallible, io, sync::Arc};
use tokio::task::JoinError;

#[derive(Clone)]
struct ArcKind {
    kind: Arc<ErrorKind>,
}

impl std::fmt::Debug for ArcKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::fmt::Display for ArcKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::error::Error for ArcKind {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

pub(crate) struct Error {
    kind: ArcKind,
    display: Box<str>,
    debug: Box<str>,
}

impl Error {
    fn kind(&self) -> &ErrorKind {
        &self.kind.kind
    }

    pub(crate) fn is_breaker(&self) -> bool {
        matches!(self.kind(), ErrorKind::Breaker)
    }

    pub(crate) fn is_not_found(&self) -> bool {
        matches!(self.kind(), ErrorKind::Status(_, StatusCode::NOT_FOUND))
    }

    pub(crate) fn is_bad_request(&self) -> bool {
        matches!(self.kind(), ErrorKind::Status(_, StatusCode::BAD_REQUEST))
    }

    pub(crate) fn is_gone(&self) -> bool {
        matches!(self.kind(), ErrorKind::Status(_, StatusCode::GONE))
    }

    pub(crate) fn is_malformed_json(&self) -> bool {
        matches!(self.kind(), ErrorKind::Json(_))
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.debug)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind().source()
    }
}

impl<T> From<T> for Error
where
    ErrorKind: From<T>,
{
    fn from(error: T) -> Self {
        let kind = ArcKind {
            kind: Arc::new(ErrorKind::from(error)),
        };
        let report = Report::new(kind.clone());
        let display = format!("{report}");
        let debug = format!("{report:?}");

        Error {
            kind,
            display: Box::from(display),
            debug: Box::from(debug),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ErrorKind {
    #[error("Error in extractor")]
    Extractor(#[from] crate::extractors::ErrorKind),

    #[error("Error queueing job")]
    Queue(#[from] BoxError),

    #[error("Error in configuration")]
    Config(#[from] config::ConfigError),

    #[error("Couldn't parse key")]
    Pkcs8(#[from] rsa::pkcs8::Error),

    #[error("Couldn't encode public key")]
    Spki(#[from] rsa::pkcs8::spki::Error),

    #[error("Couldn't sign request")]
    SignRequest,

    #[error("Couldn't make request")]
    Reqwest(#[from] reqwest::Error),

    #[error("Couldn't make request")]
    ReqwestMiddleware(#[from] reqwest_middleware::Error),

    #[error("Couldn't parse IRI")]
    ParseIri(#[from] activitystreams::iri_string::validate::Error),

    #[error("Couldn't normalize IRI")]
    NormalizeIri(#[from] std::collections::TryReserveError),

    #[error("Couldn't perform IO")]
    Io(#[from] io::Error),

    #[error("Couldn't sign string, {0}")]
    Rsa(rsa::errors::Error),

    #[error("Couldn't use db")]
    Sled(#[from] sled::Error),

    #[error("Couldn't do the json thing")]
    Json(#[from] serde_json::Error),

    #[error("Couldn't sign request")]
    Sign(#[from] SignError),

    #[error("Couldn't sign digest")]
    Signature(#[from] rsa::signature::Error),

    #[error("Couldn't prepare TLS private key")]
    PrepareKey(#[from] rustls::Error),

    #[error("Couldn't verify signature")]
    VerifySignature,

    #[error("Failed to encode key der")]
    DerEncode,

    #[error("Couldn't parse the signature header")]
    HeaderValidation(#[from] actix_web::http::header::InvalidHeaderValue),

    #[error("Couldn't decode base64")]
    Base64(#[from] base64::DecodeError),

    #[error("Actor ({0}), or Actor's server, is not subscribed")]
    NotSubscribed(String),

    #[error("Actor is not allowed, {0}")]
    NotAllowed(String),

    #[error("Cannot make decisions for foreign actor, {0}")]
    WrongActor(String),

    #[error("Actor ({0}) tried to submit another actor's ({1}) payload")]
    BadActor(String, String),

    #[error("Signature verification is required, but no signature was given")]
    NoSignature(Option<String>),

    #[error("Wrong ActivityPub kind, {0}")]
    Kind(String),

    #[error("Too many CPUs")]
    CpuCount(#[from] std::num::TryFromIntError),

    #[error("Host mismatch")]
    HostMismatch(#[from] CheckError),

    #[error("Couldn't flush buffer")]
    FlushBuffer,

    #[error("Invalid algorithm provided to verifier, {0}")]
    Algorithm(String),

    #[error("Object has already been relayed")]
    Duplicate,

    #[error("Couldn't send request to {0}, {1}")]
    SendRequest(String, String),

    #[error("Couldn't receive request response from {0}, {1}")]
    ReceiveResponse(String, String),

    #[error("Response from {0} has invalid status code, {1}")]
    Status(String, StatusCode),

    #[error("Expected an Object, found something else")]
    ObjectFormat,

    #[error("Expected a single object, found array")]
    ObjectCount,

    #[error("Input is missing a 'type' field")]
    MissingKind,

    #[error("Input is missing a 'id' field")]
    MissingId,

    #[error("IriString is missing a domain")]
    MissingDomain,

    #[error("URI is missing domain field")]
    Domain,

    #[error("Blocking operation was canceled")]
    Canceled,

    #[error("Not trying request due to failed breaker")]
    Breaker,

    #[error("Failed to extract fields from {0}")]
    Extract(&'static str),

    #[error("No API Token supplied")]
    MissingApiToken,
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self.kind() {
            ErrorKind::NotAllowed(_) | ErrorKind::WrongActor(_) | ErrorKind::BadActor(_, _) => {
                StatusCode::FORBIDDEN
            }
            ErrorKind::NotSubscribed(_) => StatusCode::UNAUTHORIZED,
            ErrorKind::Duplicate => StatusCode::ACCEPTED,
            ErrorKind::Kind(_)
            | ErrorKind::MissingKind
            | ErrorKind::MissingId
            | ErrorKind::ObjectCount
            | ErrorKind::NoSignature(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(("Content-Type", "application/activity+json"))
            .body(
                serde_json::to_string(&serde_json::json!({
                    "error": self.kind().to_string(),
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
    }
}

impl From<BlockingError> for ErrorKind {
    fn from(_: BlockingError) -> Self {
        ErrorKind::Canceled
    }
}

impl From<JoinError> for ErrorKind {
    fn from(_: JoinError) -> Self {
        ErrorKind::Canceled
    }
}

impl From<Infallible> for ErrorKind {
    fn from(i: Infallible) -> Self {
        match i {}
    }
}

impl From<rsa::errors::Error> for ErrorKind {
    fn from(e: rsa::errors::Error) -> Self {
        ErrorKind::Rsa(e)
    }
}

impl From<http_signature_normalization_actix::Canceled> for ErrorKind {
    fn from(_: http_signature_normalization_actix::Canceled) -> Self {
        Self::Canceled
    }
}

impl From<http_signature_normalization_reqwest::Canceled> for ErrorKind {
    fn from(_: http_signature_normalization_reqwest::Canceled) -> Self {
        Self::Canceled
    }
}
