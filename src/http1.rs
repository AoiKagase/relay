pub(crate) fn name_to_http02(
    name: &reqwest::header::HeaderName,
) -> actix_web::http::header::HeaderName {
    actix_web::http::header::HeaderName::from_bytes(name.as_ref())
        .expect("headername conversions always work")
}

pub(crate) fn value_to_http02(
    value: &reqwest::header::HeaderValue,
) -> actix_web::http::header::HeaderValue {
    actix_web::http::header::HeaderValue::from_bytes(value.as_bytes())
        .expect("headervalue conversions always work")
}

pub(crate) fn status_to_http02(status: reqwest::StatusCode) -> actix_web::http::StatusCode {
    actix_web::http::StatusCode::from_u16(status.as_u16())
        .expect("statuscode conversions always work")
}
