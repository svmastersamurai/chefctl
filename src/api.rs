extern crate actix_web;

use actix_web::{server, HttpRequest};

pub fn start_api_server(addr: &str) -> std::io::Result<()> {
    server::new(|| actix_web::App::new().resource("/", |r| r.f(index)))
        .bind(addr)?
        .run();

    Ok(())
}

pub fn index(_req: &HttpRequest) -> String {
    let state = crate::state::APP_STATE.peek();

    format!("{:?}\n", state).to_string()
}
