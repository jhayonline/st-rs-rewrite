use salvo::cors::Cors;
use salvo::http::Method;
use salvo::prelude::*;

pub fn cors_handler() -> impl Handler {
    Cors::new()
        .allow_origin("*")
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(vec!["content-type", "authorization"])
        .allow_credentials(false)
        .max_age(86400)
        .into_handler()
}
