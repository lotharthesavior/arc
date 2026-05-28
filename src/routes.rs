use crate::helpers::jwt::jwt_auth_enabled;
use crate::http::controllers::api_controller::{login, profile};
use crate::http::controllers::{admin_controller, auth_controller, home_controller};
use crate::http::middlewares::{auth_middleware::AuthMiddleware, jwt_middleware::JwtMiddleware};
use crate::websocket;
use actix_files as fs;
use actix_web::http::header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH};
use actix_web::{get, web, Error, HttpRequest, HttpResponse};

#[get("/public/{filename:.*}")]
pub async fn static_file(req: HttpRequest) -> Result<HttpResponse, Error> {
    let path: std::path::PathBuf = req.match_info().query("filename").parse().unwrap();
    let filename = path.to_string_lossy();

    // Check if file has a hash in the name (e.g., script-1MSs88XQ.js)
    // These are immutable and can be cached forever
    let is_hashed =
        filename.contains('-') && (filename.ends_with(".js") || filename.ends_with(".css"));

    let file = fs::NamedFile::open(std::path::Path::new("./dist").join(path.clone()))?;

    // Generate ETag from file metadata
    let metadata = file.file().metadata()?;
    let etag_value = format!(
        "{:x}-{:x}",
        metadata.len(),
        metadata
            .modified()
            .map(|t| t
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs())
            .unwrap_or(0)
    );

    // Check If-None-Match header for conditional requests
    if let Some(if_none_match) = req.headers().get(IF_NONE_MATCH) {
        if let Ok(header_value) = if_none_match.to_str() {
            if header_value.trim_matches('"') == etag_value
                || header_value == format!("\"{}\"", etag_value)
            {
                return Ok(HttpResponse::NotModified().finish());
            }
        }
    }

    let mut response = file.into_response(&req);
    let headers = response.headers_mut();

    // Add ETag header
    headers.insert(ETAG, format!("\"{}\"", etag_value).parse().unwrap());

    // Add Cache-Control header
    if is_hashed {
        // Hashed files are immutable - cache for 1 year
        headers.insert(
            CACHE_CONTROL,
            "public, max-age=31536000, immutable".parse().unwrap(),
        );
    } else {
        // Other static files - cache for 1 hour, revalidate
        headers.insert(
            CACHE_CONTROL,
            "public, max-age=3600, must-revalidate".parse().unwrap(),
        );
    }

    Ok(response)
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        // GET /home
        .service(home_controller::home)
        // GET /signin
        .service(auth_controller::signin)
        // POST /signin
        .service(auth_controller::signin_post)
        // GET /signout
        .service(auth_controller::signout)
        // GET /admin
        .service(
            web::scope("/admin")
                .wrap(AuthMiddleware)
                .service(admin_controller::dashboard)
                .service(admin_controller::settings)
                .service(admin_controller::profile)
                .service(admin_controller::profile_post)
                .service(admin_controller::profile_password_post),
        )
        // WebSocket endpoint
        .route("/ws", web::get().to(websocket::connection::ws_handler))
        .service(static_file);

    if jwt_auth_enabled() {
        cfg.service(
            web::scope("/api").service(login).service(
                web::scope("/protected")
                    .wrap(JwtMiddleware)
                    .service(profile),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use actix_web::{http, test, App};

    #[actix_web::test]
    async fn test_static_file_ok() {
        let app = test::init_service(App::new().service(static_file)).await;

        fs::create_dir_all("./dist").unwrap();
        fs::write("./dist/styles.css", "").unwrap();

        let req = test::TestRequest::get()
            .uri("/public/styles.css")
            .to_request();
        let resp = test::call_service(&app, req).await;

        fs::remove_file("./dist/styles.css").unwrap();

        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_static_file_not_found() {
        let app = test::init_service(App::new().service(static_file)).await;

        let req = test::TestRequest::get()
            .uri("/public/not-existing-styles.css")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }
}
