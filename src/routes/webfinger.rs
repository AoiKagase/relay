use crate::{
    config::{Config, UrlKind},
    data::State,
};
use actix_web::{
    dev::Payload,
    http::{header::ACCEPT, StatusCode},
    web::{Data, Query},
    FromRequest, HttpRequest, HttpResponse, ResponseError,
};
use rsa_magic_public_key::AsMagicPublicKey;
use std::future::{ready, Ready};
use url::Url;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ErrorKind {
    #[error("Accept Header is required")]
    MissingAccept,

    #[error("Unsupported accept type")]
    InvalidAccept,

    #[error("Query is malformed")]
    InvalidQuery,

    #[error("No records match the provided resource")]
    NotFound,
}

impl ResponseError for ErrorKind {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::MissingAccept | Self::InvalidAccept | Self::InvalidQuery => {
                StatusCode::BAD_REQUEST
            }
            Self::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).finish()
    }
}

#[derive(serde::Deserialize)]
struct Resource {
    resource: String,
}

pub(crate) enum WebfingerResource {
    Url(Url),
    Unknown(String),
}

fn is_supported_json(m: &mime::Mime) -> bool {
    matches!(
        (
            m.type_().as_str(),
            m.subtype().as_str(),
            m.suffix().map(|s| s.as_str()),
        ),
        ("*", "*", None)
            | ("application", "*", None)
            | ("application", "json", None)
            | ("application", "jrd", Some("json"))
    )
}

impl WebfingerResource {
    fn parse_request(req: &HttpRequest) -> Result<Self, ErrorKind> {
        let Some(accept) = req.headers().get(ACCEPT) else {
            return Err(ErrorKind::MissingAccept);
        };

        let accept_value = accept.to_str().map_err(|_| ErrorKind::InvalidAccept)?;

        let acceptable = accept_value
            .split(", ")
            .filter_map(|accept| accept.trim().parse::<mime::Mime>().ok())
            .any(|accept| is_supported_json(&accept));

        if !acceptable {
            return Err(ErrorKind::InvalidAccept);
        }

        let Resource { resource } = Query::<Resource>::from_query(req.query_string())
            .map_err(|_| ErrorKind::InvalidQuery)?
            .into_inner();

        let wr = match Url::parse(&resource) {
            Ok(url) => WebfingerResource::Url(url),
            Err(_) => WebfingerResource::Unknown(resource),
        };

        Ok(wr)
    }
}

impl FromRequest for WebfingerResource {
    type Error = ErrorKind;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(Self::parse_request(req))
    }
}

pub(crate) async fn resolve(
    config: Data<Config>,
    state: Data<State>,
    resource: WebfingerResource,
) -> Result<HttpResponse, ErrorKind> {
    match resource {
        WebfingerResource::Unknown(handle) => {
            if handle.trim_start_matches('@') == config.generate_resource() {
                return Ok(respond(&config, &state));
            }
        }
        WebfingerResource::Url(url) => match url.scheme() {
            "acct" => {
                if url.path().trim_start_matches('@') == config.generate_resource() {
                    return Ok(respond(&config, &state));
                }
            }
            "http" | "https" => {
                if url.as_str() == config.generate_url(UrlKind::Actor).as_str() {
                    return Ok(respond(&config, &state));
                }
            }
            _ => return Err(ErrorKind::NotFound),
        },
    }

    Err(ErrorKind::NotFound)
}

fn respond(config: &Config, state: &State) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/jrd+json")
        .json(serde_json::json!({
            "subject": format!("acct:{}", config.generate_resource()),
            "aliases": [
                config.generate_url(UrlKind::Actor),
            ],
            "links": [
                {
                    "rel": "self",
                    "href": config.generate_url(UrlKind::Actor),
                    "type": "application/activity+json"
                },
                {
                    "rel": "self",
                    "href": config.generate_url(UrlKind::Actor),
                    "type": "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\""
                },
                {
                    "rel": "magic-public-key",
                    "href": state.public_key.as_magic_public_key()
                }
            ]
        }))
}
