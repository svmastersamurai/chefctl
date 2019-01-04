use actix_web::{server, HttpRequest};
use serde_json::Error;

pub fn start_api_server(addr: &str) -> std::io::Result<()> {
    server::new(|| actix_web::App::new().resource("/", |r| r.f(index)))
        .bind(addr)?
        .run();

    Ok(())
}

pub fn index(_req: &HttpRequest) -> String {
    let state_json = match serde_json::to_string_pretty(&*crate::state::APP_STATE) {
        Ok(v) => v,
        Err(e) => {
            println!("error serializing state: {}", e);

            "{\"error\": \"serialization error!\"}".into()
        }
    };

    format!("{}\n", state_json).to_string();
}
