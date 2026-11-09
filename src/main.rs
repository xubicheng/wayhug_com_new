mod admin;
mod config;
mod db;
mod i18n;
mod mail;
mod md;
mod models;
mod public;
mod reader_notes;
mod review;
mod subscribe;
mod ui;
mod util;

use std::sync::Arc;

use axum::Router;
use axum::response::Redirect;
use axum::http::header::{HeaderName, HeaderValue};
use axum::routing::{get, post};
use sqlx::SqlitePool;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

use config::Config;
use mail::Mailer;
use util::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub cfg: Arc<Config>,
    pub mailer: Arc<Mailer>,
    pub limiter: Arc<RateLimiter>,
}

fn hdr(name: &'static str, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        HeaderName::from_static(name),
        HeaderValue::from_static(value),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wayhug_new=info,tower_http=info".into()),
        )
        .init();

    let cfg = Arc::new(Config::from_env()?);
    let pool = db::init(&cfg.database_url).await?;
    let mailer = Arc::new(Mailer::new(&cfg, pool.clone()));
    let state = AppState {
        db: pool,
        cfg: cfg.clone(),
        mailer: mailer.clone(),
        limiter: Arc::new(RateLimiter::new()),
    };

    let app = Router::new()
        .route("/", get(public::root))
        .route("/healthz", get(public::healthz))
        .route("/robots.txt", get(public::robots))
        .route(
            "/favicon.ico",
            get(|| async { Redirect::permanent("/static/favicon.svg") }),
        )
        .route("/sitemap.xml", get(public::sitemap))
        .route("/api/notes", get(public::api_notes))
        .route("/{lang}/", get(public::home))
        .route("/{lang}/notes", get(public::notes_index))
        .route("/{lang}/notes/{slug}", get(public::note_detail))
        .route("/{lang}/readers", get(public::readers_index))
        .route("/{lang}/readers/{slug}", get(public::reader_detail))
        .route("/{lang}/reader-notes", get(reader_notes::index))
        .route(
            "/{lang}/reader-notes/submit",
            get(reader_notes::submit_page).post(reader_notes::submit),
        )
        .route("/{lang}/reader-notes/{slug}", get(reader_notes::detail))
        .route("/{lang}/unsubscribe", get(reader_notes::unsubscribe_page))
        .route("/{lang}/about", get(public::about))
        .route("/{lang}/products", get(public::products))
        .route(
            "/{lang}/subscribe",
            get(subscribe::subscribe_page).post(subscribe::subscribe_submit),
        )
        .route("/{lang}/subscribe/confirm", get(subscribe::confirm))
        .route("/{lang}/subscribe/done", get(subscribe::done))
        .route("/{lang}/subscribe/manage", get(subscribe::manage))
        .route("/{lang}/subscribe/unsubscribe", post(subscribe::unsubscribe))
        .route("/{lang}/subscribe/delete", post(subscribe::delete_data))
        .route(
            "/{lang}/contribute",
            get(subscribe::contribute_page).post(subscribe::contribute_submit),
        )
        .route("/admin/login", get(admin::login_page).post(admin::login_submit))
        .route("/admin/logout", post(admin::logout))
        .route("/admin", get(admin::dashboard))
        .route("/admin/notes", get(admin::notes_list).post(admin::note_create))
        .route("/admin/notes/new", get(admin::note_new))
        .route(
            "/admin/notes/{id}",
            get(admin::note_edit).post(admin::note_update),
        )
        .route("/admin/notes/{id}/delete", post(admin::note_delete))
        .route("/admin/subscribers", get(admin::subscribers))
        .route(
            "/admin/subscribers/{id}/delete",
            post(admin::subscriber_delete),
        )
        .route("/admin/authors", get(admin::authors).post(admin::author_create))
        .route("/admin/authors/status", post(admin::author_status))
        .route("/admin/authors/{id}/invite", post(admin::author_invite))
        .route("/admin/submissions", get(admin::submissions))
        .route(
            "/admin/submissions/{id}/approve",
            post(admin::submission_approve),
        )
        .route(
            "/admin/submissions/{id}/reject",
            post(admin::submission_reject),
        )
        .route("/admin/products", get(admin::products).post(admin::product_create))
        .route("/admin/products/new", get(admin::product_new))
        .route(
            "/admin/products/{id}",
            get(admin::product_edit).post(admin::product_update),
        )
        .route("/admin/review", get(review::workbench))
        .route("/admin/review/{id}/decide", post(review::decide))
        .route("/admin/review/batch-reject", post(review::batch_reject))
        .route("/admin/review/preview", post(review::preview))
        .route("/admin/audit", get(admin::audit_page))
        .route("/admin/outbox", get(admin::outbox))
        .route("/admin/outbox/{id}", get(admin::outbox_view))
        .nest_service("/static", ServeDir::new("static"))
        .layer(CompressionLayer::new())
        .layer(hdr(
            "content-security-policy",
            "default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; form-action 'self'; base-uri 'self'; frame-ancestors 'none'",
        ))
        .layer(hdr("x-content-type-options", "nosniff"))
        .layer(hdr("x-frame-options", "DENY"))
        .layer(hdr("referrer-policy", "strict-origin-when-cross-origin"))
        .layer(hdr(
            "permissions-policy",
            "geolocation=(), camera=(), microphone=(), payment=()",
        ))
        .layer(hdr("strict-transport-security", "max-age=15552000"))
        .with_state(state);

    let addr = format!("{}:{}", cfg.bind, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("wayhug.com new site listening on {}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
