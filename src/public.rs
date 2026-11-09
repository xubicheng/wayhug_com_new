use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use maud::html;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::AppState;
use crate::i18n::Lang;
use crate::models::{NoteCard, NoteFull, ProductRow, PublicNote};
use crate::ui;
use crate::util::AppError;

pub const NOTES_SQL: &str = "
SELECT n.id, n.slug,
       COALESCE(t.title, d.title, '') AS title,
       COALESCE(t.summary, d.summary, '') AS summary,
       n.published_at,
       COALESCE(d.lang, n.lang) AS content_lang,
       CASE WHEN t.lang IS NULL THEN 0 ELSE 1 END AS has_lang,
       g.name AS author_name,
       n.guest_author_id
FROM notes n
LEFT JOIN note_i18n t ON t.note_id = n.id AND t.lang = ?
LEFT JOIN note_i18n d ON d.note_id = n.id AND d.lang = n.lang
LEFT JOIN guest_authors g ON g.id = n.guest_author_id
WHERE n.status = 'published'
";

pub fn csrf_for(headers: &HeaderMap) -> (String, Option<String>) {
    match crate::util::cookie(headers, "wh_csrf") {
        Some(v) if v.len() == 32 => (v, None),
        _ => {
            let t = crate::util::token(16);
            (t.clone(), Some(t))
        }
    }
}

pub fn csrf_cookie(value: &str, secure: bool) -> String {
    format!(
        "wh_csrf={}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly{}",
        value,
        if secure { "; Secure" } else { "" }
    )
}

pub fn csrf_ok(headers: &HeaderMap, form_token: &str) -> bool {
    match crate::util::cookie(headers, "wh_csrf") {
        Some(v) => !form_token.is_empty() && crate::util::ct_eq(&v, form_token),
        None => false,
    }
}

pub fn page_response(markup: maud::Markup, new_csrf: Option<String>, st: &AppState) -> Response {
    let mut resp = Html(markup.into_string()).into_response();
    if let Some(v) = new_csrf {
        if let Ok(hv) = HeaderValue::from_str(&csrf_cookie(&v, st.cfg.cookie_secure)) {
            resp.headers_mut().append(header::SET_COOKIE, hv);
        }
    }
    resp
}

pub fn lang_or_404(lang: &str) -> Option<Lang> {
    Lang::from_seg(lang)
}

pub fn not_found_page(lang: Lang, site_url: &str) -> Response {
    let s = lang.s();
    let path = format!("/{}/404", lang.code());
    let content = html! {
        section class="narrow" {
            h1 { (s.not_found_title) }
            p { (s.not_found_body) }
            p { a class="btn" href={(format!("/{}/", lang.code()))} { (s.back_home) } }
        }
    };
    (
        StatusCode::NOT_FOUND,
        Html(ui::page(lang, &path, s.not_found_title, s.not_found_body, "", site_url, content).into_string()),
    )
        .into_response()
}

pub async fn load_notes(
    db: &SqlitePool,
    lang: Lang,
    section: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<NoteCard>, sqlx::Error> {
    let filter = match section {
        "readers" => " AND n.guest_author_id IS NOT NULL",
        "reader-notes" => {
            " AND EXISTS (SELECT 1 FROM reader_note_meta m WHERE m.note_id = n.id)"
        }
        _ => {
            " AND n.guest_author_id IS NULL
              AND NOT EXISTS (SELECT 1 FROM reader_note_meta m WHERE m.note_id = n.id)"
        }
    };
    let sql = format!(
        "{} {} ORDER BY n.published_at DESC, n.id DESC LIMIT ? OFFSET ?",
        NOTES_SQL, filter
    );
    sqlx::query_as::<_, NoteCard>(&sql)
        .bind(lang.code())
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
}

async fn load_products(db: &SqlitePool, lang: Lang) -> Result<Vec<ProductRow>, sqlx::Error> {
    sqlx::query_as::<_, ProductRow>(
        "SELECT p.id, p.slug,
                COALESCE(t.name, d.name, '') AS name,
                COALESCE(t.description, d.description, '') AS description,
                COALESCE(t.revenue_note, d.revenue_note, '') AS revenue_note,
                p.price_cents, p.currency, p.stock,
                CASE WHEN t.lang IS NULL THEN 0 ELSE 1 END AS has_lang
         FROM products p
         LEFT JOIN product_i18n t ON t.product_id = p.id AND t.lang = ?
         LEFT JOIN product_i18n d ON d.product_id = p.id AND d.lang = 'zh'
         ORDER BY p.id",
    )
    .bind(lang.code())
    .fetch_all(db)
    .await
}

pub async fn home(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let Some(lang) = lang_or_404(&lang) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let notes = load_notes(&st.db, lang, "notes", 4, 0).await?;
    let readers = load_notes(&st.db, lang, "readers", 2, 0).await?;
    let products = load_products(&st.db, lang).await?;
    let (csrf, new) = csrf_for(&headers);
    let notes_href = format!("/{}/notes", lang.code());
    let readers_href = format!("/{}/readers", lang.code());
    let products_href = format!("/{}/products", lang.code());
    let subscribe_href = format!("/{}/subscribe", lang.code());

    let content = html! {
        (ui::hero(lang))
        section class="block" {
            (ui::section_head(lang, s.home_notes_title, Some(notes_href.as_str()), s.home_notes_all))
            (ui::note_list(lang, &notes, "notes"))
        }
        section class="block split" {
            div {
                h2 { (s.home_why_title) }
                ul class="plain-list" {
                    li { (s.home_why_1) }
                    li { (s.home_why_2) }
                    li { (s.home_why_3) }
                }
            }
            div class="form-box" {
                (ui::subscribe_form(lang, &csrf, &subscribe_href))
            }
        }
        section class="block" {
            (ui::section_head(lang, s.home_readers_title, Some(readers_href.as_str()), s.nav_readers))
            (ui::note_list(lang, &readers, "readers"))
        }
        section class="block" {
            (ui::section_head(lang, s.home_products_title, Some(products_href.as_str()), s.nav_products))
            p class="section-note" { (s.home_products_note) }
            div class="product-grid" {
                @for p in products.iter().take(2) {
                    (ui::product_card(lang, p))
                }
            }
        }
    };

    let path = format!("/{}/", lang.code());
    Ok(page_response(
        ui::page(lang, &path, "", s.site_desc, "", &st.cfg.site_url, content),
        new,
        &st,
    ))
}

#[derive(Deserialize)]
pub struct PageQuery {
    pub page: Option<u32>,
}

pub async fn notes_index(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    Query(q): Query<PageQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let Some(lang) = lang_or_404(&lang) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let page = q.page.unwrap_or(1).max(1);
    let limit = 10_i64;
    let offset = (page as i64 - 1) * limit;
    let mut notes = load_notes(&st.db, lang, "notes", limit + 1, offset).await?;
    let has_next = notes.len() as i64 > limit;
    if has_next {
        notes.truncate(limit as usize);
    }
    let (csrf, new) = csrf_for(&headers);
    let subscribe_href = format!("/{}/subscribe", lang.code());
    let next_href = format!("/{}/notes?page={}", lang.code(), page + 1);
    let content = html! {
        section class="narrow" {
            h1 { (s.notes_title) }
            p class="lead" { (s.notes_intro) }
            (ui::note_list(lang, &notes, "notes"))
            @if has_next || page > 1 {
                nav class="pager" {
                    @if page > 1 {
                        a href={(format!("/{}/notes?page={}", lang.code(), page - 1))} { "← " (s.previous_note) }
                    }
                    @if has_next {
                        a href=(next_href) { (s.next_note) " →" }
                    }
                }
            }
            div class="form-box form-box--slim" {
                (ui::subscribe_form(lang, &csrf, &subscribe_href))
            }
        }
    };
    let path = format!("/{}/notes", lang.code());
    Ok(page_response(
        ui::page(lang, &path, s.notes_title, s.notes_intro, "notes", &st.cfg.site_url, content),
        new,
        &st,
    ))
}

async fn load_note(db: &SqlitePool, lang: Lang, slug: &str) -> Result<Option<NoteFull>, sqlx::Error> {
    sqlx::query_as::<_, NoteFull>(
        "SELECT n.id, n.slug,
                COALESCE(t.title, d.title, '') AS title,
                COALESCE(t.summary, d.summary, '') AS summary,
                COALESCE(t.body, d.body, '') AS body,
                n.published_at,
                COALESCE(d.lang, n.lang) AS content_lang,
                CASE WHEN t.lang IS NULL THEN 0 ELSE 1 END AS has_lang,
                n.tags, n.license,
                g.name AS author_name, g.bio AS author_bio,
                n.guest_author_id
         FROM notes n
         LEFT JOIN note_i18n t ON t.note_id = n.id AND t.lang = ?
         LEFT JOIN note_i18n d ON d.note_id = n.id AND d.lang = n.lang
         LEFT JOIN guest_authors g ON g.id = n.guest_author_id
         WHERE n.slug = ? AND n.status = 'published'",
    )
    .bind(lang.code())
    .bind(slug)
    .fetch_optional(db)
    .await
}

async fn neighbor(
    db: &SqlitePool,
    lang: Lang,
    published_at: &str,
    next: bool,
    section: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let cmp = if next { ">" } else { "<" };
    let order = if next { "ASC" } else { "DESC" };
    let filter = match section {
        "readers" => " AND n.guest_author_id IS NOT NULL",
        "reader-notes" => {
            " AND EXISTS (SELECT 1 FROM reader_note_meta m WHERE m.note_id = n.id)"
        }
        _ => {
            " AND n.guest_author_id IS NULL
              AND NOT EXISTS (SELECT 1 FROM reader_note_meta m WHERE m.note_id = n.id)"
        }
    };
    let sql = format!(
        "SELECT n.slug, COALESCE(t.title, d.title, '') AS title
         FROM notes n
         LEFT JOIN note_i18n t ON t.note_id = n.id AND t.lang = ?
         LEFT JOIN note_i18n d ON d.note_id = n.id AND d.lang = n.lang
         WHERE n.status = 'published' AND n.published_at {} ? {}
         ORDER BY n.published_at {} LIMIT 1",
        cmp, filter, order
    );
    sqlx::query_as::<_, (String, String)>(&sql)
        .bind(lang.code())
        .bind(published_at)
        .fetch_optional(db)
        .await
}

pub async fn note_detail(
    State(st): State<AppState>,
    Path((lang, slug)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    detail_impl(st, lang, slug, headers, "notes").await
}

pub async fn reader_detail(
    State(st): State<AppState>,
    Path((lang, slug)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    detail_impl(st, lang, slug, headers, "readers").await
}

pub async fn detail_impl(
    st: AppState,
    lang_raw: String,
    slug: String,
    headers: HeaderMap,
    section: &str,
) -> Result<Response, AppError> {
    let Some(lang) = lang_or_404(&lang_raw) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let Some(note) = load_note(&st.db, lang, &slug).await? else {
        return Ok(not_found_page(lang, &st.cfg.site_url));
    };
    let meta = crate::reader_notes::load_meta(&st.db, note.id).await?;
    let is_reader_note = meta.is_some();
    let fits = match section {
        "readers" => note.guest_author_id.is_some() && !is_reader_note,
        "reader-notes" => is_reader_note,
        _ => note.guest_author_id.is_none() && !is_reader_note,
    };
    if !fits {
        return Ok(not_found_page(lang, &st.cfg.site_url));
    }

    let (csrf, new) = csrf_for(&headers);
    let subscribe_href = format!("/{}/subscribe", lang.code());
    let body_html = crate::md::render(&note.body);
    let minutes = crate::util::reading_minutes(&note.body);
    let published = note.published_at.as_deref().unwrap_or("");
    let prev = neighbor(&st.db, lang, published, false, section).await?;
    let next = neighbor(&st.db, lang, published, true, section).await?;
    let tags: Vec<&str> = note.tags.split(',').map(|t| t.trim()).filter(|t| !t.is_empty()).collect();
    let kicker = match section {
        "readers" => s.nav_readers,
        "reader-notes" => s.reader_note_kicker,
        _ => s.nav_notes,
    };

    let content = html! {
        article class="article" {
            header class="article-head" {
                p class="kicker" { (kicker) }
                h1 { (note.title) }
                p class="note-meta" {
                    time datetime=(published) { (lang.date(published)) }
                    span class="dot" { "·" }
                    span { (lang.reading_time(minutes)) }
                    @if let Some(name) = &note.author_name {
                        span class="dot" { "·" }
                        span { (name) }
                    }
                }
                @if let Some(bio) = &note.author_bio {
                    @if !bio.is_empty() {
                        p class="author-bio" { (bio) }
                    }
                }
                @if note.has_lang == 0 && note.content_lang != lang.code() {
                    (ui::notice("warn", s.fallback_notice))
                }
            }
            @if let Some(m) = &meta {
                (crate::reader_notes::meta_block(lang, m))
            }
            (ui::prose(&body_html))
            @if let Some(m) = &meta {
                @if !m.question.is_empty() {
                    div class="rn-question" {
                        h2 { (s.reader_note_question) }
                        p { (m.question) }
                    }
                }
            }
            @if !tags.is_empty() {
                p class="tags" {
                    span class="tags-label" { (s.tags_label) "：" }
                    @for t in &tags {
                        span class="tag" { (t) }
                    }
                }
            }
            @if section == "readers" {
                p class="license" { (note.license.clone().unwrap_or_else(|| s.readers_license.to_string())) }
            }
            nav class="pager" {
                @if let Some((slug, title)) = &prev {
                    a href={(format!("/{}/{}/{}", lang.code(), section, slug))} { "← " (title) }
                }
                @if let Some((slug, title)) = &next {
                    a href={(format!("/{}/{}/{}", lang.code(), section, slug))} { (title) " →" }
                }
            }
        }
        div class="form-box form-box--slim" {
            h2 { (s.subscribe_title) }
            (ui::subscribe_form(lang, &csrf, &subscribe_href))
        }
    };

    let path = format!("/{}/{}/{}", lang.code(), section, note.slug);
    Ok(page_response(
        ui::page(lang, &path, &note.title, &note.summary, section, &st.cfg.site_url, content),
        new,
        &st,
    ))
}

pub async fn readers_index(
    State(st): State<AppState>,
    Path(lang): Path<String>,
) -> Result<Response, AppError> {
    let Some(lang) = lang_or_404(&lang) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let notes = load_notes(&st.db, lang, "readers", 50, 0).await?;
    let content = html! {
        section class="narrow" {
            h1 { (s.readers_title) }
            p class="lead" { (s.readers_intro) }
            (ui::note_list(lang, &notes, "readers"))
        }
    };
    let path = format!("/{}/readers", lang.code());
    Ok(page_response(
        ui::page(lang, &path, s.readers_title, s.readers_intro, "readers", &st.cfg.site_url, content),
        None,
        &st,
    ))
}

pub async fn about(
    State(st): State<AppState>,
    Path(lang): Path<String>,
) -> Result<Response, AppError> {
    let Some(lang) = lang_or_404(&lang) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let mail = format!("mailto:{}", st.cfg.contact_email);
    let content = html! {
        section class="narrow prose-about" {
            h1 { (s.about_title) }
            p class="lead" { (s.about_lead) }
            p { (s.about_p1) }
            p { (s.about_p2) }
            h2 { (s.about_money_title) }
            ul class="plain-list" {
                li { (s.about_money_1) }
                li { (s.about_money_2) }
                li { (s.about_money_3) }
            }
            h2 id="privacy" { (s.about_privacy_title) }
            p { (s.about_privacy) }
            h2 { (s.about_contact_title) }
            p { (s.about_contact) }
            p { a class="link" href=(mail) { (st.cfg.contact_email) } }
        }
    };
    let path = format!("/{}/about", lang.code());
    Ok(page_response(
        ui::page(lang, &path, s.about_title, s.about_lead, "about", &st.cfg.site_url, content),
        None,
        &st,
    ))
}

pub async fn products(
    State(st): State<AppState>,
    Path(lang): Path<String>,
) -> Result<Response, AppError> {
    let Some(lang) = lang_or_404(&lang) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let items = load_products(&st.db, lang).await?;
    let mail = format!("mailto:{}?subject=wayhug", st.cfg.contact_email);
    let content = html! {
        section class="narrow" {
            h1 { (s.products_title) }
            p class="lead" { (s.products_intro) }
            @if items.is_empty() {
                p { (s.products_empty) }
            }
            div class="product-grid" {
                @for p in &items {
                    (ui::product_card(lang, p))
                }
            }
            @if !items.is_empty() {
                p class="order-note" {
                    (s.order_note) " "
                    a class="link" href=(mail) { (st.cfg.contact_email) }
                }
            }
        }
    };
    let path = format!("/{}/products", lang.code());
    Ok(page_response(
        ui::page(lang, &path, s.products_title, s.products_intro, "products", &st.cfg.site_url, content),
        None,
        &st,
    ))
}

pub async fn api_notes(State(st): State<AppState>) -> Result<Response, AppError> {
    let rows: Vec<(String, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT n.slug, i.lang, i.title, i.summary, n.published_at
         FROM notes n JOIN note_i18n i ON i.note_id = n.id
         WHERE n.status = 'published'
         ORDER BY n.published_at DESC, n.id DESC",
    )
    .fetch_all(&st.db)
    .await?;
    let out: Vec<PublicNote> = rows
        .into_iter()
        .map(|(slug, lang, title, summary, published_at)| PublicNote {
            slug,
            lang,
            title,
            summary,
            published_at: published_at.unwrap_or_default(),
        })
        .collect();
    Ok(axum::Json(out).into_response())
}

pub async fn healthz(State(st): State<AppState>) -> Response {
    match sqlx::query_scalar::<_, i64>("SELECT 1").fetch_one(&st.db).await {
        Ok(_) => (StatusCode::OK, "ok").into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "db error").into_response(),
    }
}

pub async fn robots() -> Response {
    let body = "User-agent: *\nAllow: /\nSitemap: https://wayhug.com/sitemap.xml\n";
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body).into_response()
}

pub async fn sitemap(State(st): State<AppState>) -> Result<Response, AppError> {
    let rows: Vec<(String, Option<String>, i64)> = sqlx::query_as(
        "SELECT n.slug, n.published_at,
                CASE WHEN EXISTS (SELECT 1 FROM reader_note_meta m WHERE m.note_id = n.id)
                     THEN 1 ELSE 0 END AS is_reader_note
         FROM notes n WHERE n.status = 'published' ORDER BY n.published_at DESC",
    )
    .fetch_all(&st.db)
    .await?;
    let base = st.cfg.site_url.trim_end_matches('/');
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for lang in ["zh", "en"] {
        for path in ["", "notes", "reader-notes", "readers", "products", "about", "subscribe"] {
            let loc = format!("{}/{}/{}", base, lang, path);
            xml.push_str(&format!("<url><loc>{}</loc></url>\n", loc));
        }
        for (slug, _, is_reader_note) in &rows {
            let section = if *is_reader_note == 1 { "reader-notes" } else { "notes" };
            xml.push_str(&format!(
                "<url><loc>{}/{}/{}/{}</loc></url>\n",
                base, lang, section, slug
            ));
        }
    }
    xml.push_str("</urlset>\n");
    Ok(([(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml).into_response())
}

pub async fn root() -> Redirect {
    Redirect::temporary("/zh/")
}
