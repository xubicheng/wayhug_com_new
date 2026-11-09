use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use maud::{DOCTYPE, Markup, html};
use serde::Deserialize;

use crate::AppState;
use crate::models::{AdminNoteRow, AuditRow, GuestAuthorRow, MailRow, SessionRow, SubscriberRow, SubmissionRow};
use crate::public::{csrf_cookie, csrf_for, csrf_ok};
use crate::ui;
use crate::util::{self, AppError, sha256_hex};

const ADMIN_COOKIE: &str = "wh_admin";

pub(crate) async fn admin_page(
    st: &AppState,
    title: &str,
    active: &str,
    csrf: &str,
    body: Markup,
) -> Markup {
    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM note_submissions WHERE status = 'pending'")
            .fetch_one(&st.db)
            .await
            .unwrap_or(0);
    let nav = [
        ("", "概览", "/admin"),
        ("notes", "笔记", "/admin/notes"),
        ("review", "读者笔记审阅", "/admin/review"),
        ("subscribers", "订阅者", "/admin/subscribers"),
        ("authors", "受邀作者", "/admin/authors"),
        ("submissions", "受邀投稿", "/admin/submissions"),
        ("products", "文创", "/admin/products"),
        ("audit", "审计日志", "/admin/audit"),
        ("outbox", "发件箱", "/admin/outbox"),
    ];
    html! {
        (DOCTYPE)
        html lang="zh-CN" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="robots" content="noindex";
                title { (title) " · 后台 · wayhug.com" }
                link rel="stylesheet" href="/static/site.css";
            }
            body class="admin" {
                header class="admin-bar" {
                    div class="admin-brand" { "wayhug.com 后台" }
                    nav class="admin-nav" {
                        @for (key, label, href) in nav {
                            a class=(if active == key { "active" } else { "" }) href=(href) {
                                (label)
                                @if key == "review" && pending > 0 {
                                    span class="queue-badge" { (pending) }
                                }
                            }
                        }
                    }
                    form method="post" action="/admin/logout" class="logout" {
                        input type="hidden" name="csrf" value=(csrf);
                        button type="submit" { "退出" }
                    }
                }
                main class="admin-main" { (body) }
            }
        }
    }
}

fn login_box(error: Option<&str>, csrf: &str) -> Markup {
    html! {
        (DOCTYPE)
        html lang="zh-CN" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="robots" content="noindex";
                title { "登录 · wayhug.com 后台" }
                link rel="stylesheet" href="/static/site.css";
            }
            body class="admin admin-login" {
                main class="login-box" {
                    h1 { "站长登录" }
                    @if let Some(e) = error {
                        p class="notice warn" { (e) }
                    }
                    form method="post" action="/admin/login" {
                        input type="hidden" name="csrf" value=(csrf);
                        div class="field" {
                            label for="u" { "用户名" }
                            input id="u" name="username" required autocomplete="username";
                        }
                        div class="field" {
                            label for="p" { "密码" }
                            input id="p" name="password" type="password" required autocomplete="current-password";
                        }
                        button class="btn" type="submit" { "登录" }
                    }
                    p class="hint" { "仅站长单人使用。连续失败会触发限流。登录行为写入审计日志。" }
                }
            }
        }
    }
}

async fn find_session(st: &AppState, headers: &HeaderMap) -> Option<SessionRow> {
    let token = util::cookie(headers, ADMIN_COOKIE)?;
    let hash = sha256_hex(&token);
    let row = sqlx::query_as::<_, SessionRow>(
        "SELECT id, csrf, expires_at FROM sessions WHERE token_hash = ?",
    )
    .bind(hash)
    .fetch_optional(&st.db)
    .await
    .ok()??;
    if row.expires_at < util::now_utc() {
        let _ = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(row.id)
            .execute(&st.db)
            .await;
        return None;
    }
    Some(row)
}

pub(crate) async fn guard(st: &AppState, headers: &HeaderMap) -> Result<SessionRow, Response> {
    match find_session(st, headers).await {
        Some(s) => Ok(s),
        None => Err((
            StatusCode::FOUND,
            [(header::LOCATION, "/admin/login")],
        )
            .into_response()),
    }
}

pub(crate) async fn audit(
    db: &sqlx::SqlitePool,
    action: &str,
    target: &str,
    detail: &str,
    ip: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO audit_log (at, actor, action, target, detail, ip) VALUES (?, 'admin', ?, ?, ?, ?)",
    )
    .bind(util::now_utc())
    .bind(action)
    .bind(target)
    .bind(detail)
    .bind(ip)
    .execute(db)
    .await;
}

fn session_cookie(token: &str, secure: bool) -> String {
    format!(
        "{}={}; Path=/; Max-Age=43200; HttpOnly; SameSite=Strict{}",
        ADMIN_COOKIE,
        token,
        if secure { "; Secure" } else { "" }
    )
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: Option<String>,
    pub password: Option<String>,
    pub csrf: String,
}

pub async fn login_page(State(st): State<AppState>, headers: HeaderMap) -> Response {
    if find_session(&st, &headers).await.is_some() {
        return Redirect::to("/admin").into_response();
    }
    let (csrf, new) = csrf_for(&headers);
    let mut resp = Html(login_box(None, &csrf).into_string()).into_response();
    if let Some(v) = new {
        if let Ok(hv) = HeaderValue::from_str(&csrf_cookie(&v, st.cfg.cookie_secure)) {
            resp.headers_mut().append(header::SET_COOKIE, hv);
        }
    }
    resp
}

pub async fn login_submit(
    State(st): State<AppState>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<LoginForm>,
) -> Result<Response, AppError> {
    let ip = util::client_ip(&headers);
    if !st
        .limiter
        .allow(&format!("login:{}", ip), 5, std::time::Duration::from_secs(900))
    {
        return Ok((StatusCode::TOO_MANY_REQUESTS, "尝试过多，请 15 分钟后再试").into_response());
    }
    let (csrf, new) = csrf_for(&headers);
    if !csrf_ok(&headers, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let user = form.username.unwrap_or_default();
    let pass = form.password.unwrap_or_default();
    let ok = util::ct_eq(&user, &st.cfg.admin_user) & util::ct_eq(&pass, &st.cfg.admin_password);
    if !ok {
        audit(&st.db, "login_failed", &user, "", &ip).await;
        let mut resp = Html(login_box(Some("用户名或密码不正确。"), &csrf).into_string()).into_response();
        if let Some(v) = new {
            if let Ok(hv) = HeaderValue::from_str(&csrf_cookie(&v, st.cfg.cookie_secure)) {
                resp.headers_mut().append(header::SET_COOKIE, hv);
            }
        }
        return Ok(resp);
    }

    let token = util::token(32);
    let csrf_token = util::token(16);
    sqlx::query(
        "INSERT INTO sessions (token_hash, csrf, created_at, expires_at, last_seen_at, ip)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(sha256_hex(&token))
    .bind(&csrf_token)
    .bind(util::now_utc())
    .bind(util::plus_hours(12))
    .bind(util::now_utc())
    .bind(&ip)
    .execute(&st.db)
    .await?;
    audit(&st.db, "login", &user, "", &ip).await;

    let mut resp = Redirect::to("/admin").into_response();
    resp.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie(&token, st.cfg.cookie_secure)).unwrap(),
    );
    Ok(resp)
}

#[derive(Deserialize)]
pub struct CsrfForm {
    pub csrf: String,
}

pub async fn logout(
    State(st): State<AppState>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<CsrfForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(sess.id)
        .execute(&st.db)
        .await?;
    audit(&st.db, "logout", "", "", &util::client_ip(&headers)).await;
    let mut resp = Redirect::to("/admin/login").into_response();
    resp.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!("{}=; Path=/; Max-Age=0", ADMIN_COOKIE)).unwrap(),
    );
    Ok(resp)
}

pub async fn dashboard(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let notes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE status = 'published'")
        .fetch_one(&st.db)
        .await?;
    let drafts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE status = 'draft'")
        .fetch_one(&st.db)
        .await?;
    let confirmed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM subscribers WHERE status = 'confirmed'")
            .fetch_one(&st.db)
            .await?;
    let pending_subs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM subscribers WHERE status = 'pending'")
            .fetch_one(&st.db)
            .await?;
    let pending_subs2: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM submissions WHERE status = 'pending'")
            .fetch_one(&st.db)
            .await?;
    let products: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products")
        .fetch_one(&st.db)
        .await?;
    let audits: Vec<AuditRow> = sqlx::query_as(
        "SELECT at, actor, action, target, detail, ip FROM audit_log ORDER BY id DESC LIMIT 10",
    )
    .fetch_all(&st.db)
    .await?;

    let body = html! {
        h1 { "概览" }
        div class="stat-grid" {
            div class="stat" { strong { (notes) } span { "已发布笔记" } }
            div class="stat" { strong { (drafts) } span { "草稿" } }
            div class="stat" { strong { (confirmed) } span { "已确认订阅" } }
            div class="stat" { strong { (pending_subs) } span { "等待确认" } }
            div class="stat" { strong { (pending_subs2) } span { "待审投稿" } }
            div class="stat" { strong { (products) } span { "文创条目" } }
        }
        p class="hint" {
            "邮件状态："
            @if st.mailer.smtp_enabled { "SMTP 已配置，确认邮件会真实投递。" }
            @else { "SMTP 未配置，邮件只写入发件箱与日志。" }
        }
        h2 { "最近操作" }
        table class="table" {
            thead { tr { th { "时间" } th { "动作" } th { "对象" } th { "IP" } } }
            tbody {
                @for a in &audits {
                    tr { td { (a.at) } td { (a.action) } td { (a.target.clone().unwrap_or_default()) } td { (a.ip.clone().unwrap_or_default()) } }
                }
            }
        }
    };
    Ok(Html(admin_page(&st, "概览", "", &sess.csrf, body).await.into_string()).into_response())
}

pub async fn notes_list(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let rows: Vec<AdminNoteRow> = sqlx::query_as(
        "SELECT n.id, n.slug, n.status, n.published_at, n.updated_at, n.guest_author_id,
                (SELECT title FROM note_i18n WHERE note_id = n.id AND lang = 'zh') AS zh_title,
                (SELECT title FROM note_i18n WHERE note_id = n.id AND lang = 'en') AS en_title
         FROM notes n ORDER BY n.published_at DESC, n.id DESC",
    )
    .fetch_all(&st.db)
    .await?;
    let body = html! {
        div class="admin-head" {
            h1 { "笔记" }
            a class="btn" href="/admin/notes/new" { "新建笔记" }
        }
        table class="table" {
            thead { tr { th { "ID" } th { "slug" } th { "中文标题" } th { "英文标题" } th { "状态" } th { "日期" } th { "" } } }
            tbody {
                @for n in &rows {
                    tr {
                        td { (n.id) }
                        td { (n.slug) }
                        td { (n.zh_title.clone().unwrap_or_else(|| "—".into())) }
                        td { (n.en_title.clone().unwrap_or_else(|| "—".into())) }
                        td { (if n.status == "published" { "已发布" } else { "草稿" }) }
                        td { (n.published_at.clone().unwrap_or_default()) }
                        td { a href={(format!("/admin/notes/{}", n.id))} { "编辑" } }
                    }
                }
            }
        }
    };
    Ok(Html(admin_page(&st, "笔记", "notes", &sess.csrf, body).await.into_string()).into_response())
}

#[derive(Deserialize)]
pub struct NoteForm {
    pub id: Option<i64>,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub published_at: Option<String>,
    pub tags: Option<String>,
    pub license: Option<String>,
    pub guest_author_id: Option<i64>,
    pub zh_title: Option<String>,
    pub zh_summary: Option<String>,
    pub zh_body: Option<String>,
    pub en_title: Option<String>,
    pub en_summary: Option<String>,
    pub en_body: Option<String>,
    pub csrf: String,
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if (c == ' ' || c == '-' || c == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        format!("note-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"))
    } else {
        trimmed.chars().take(80).collect()
    }
}

fn valid_date(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        Some(s.to_string())
    } else {
        None
    }
}

async fn save_note(st: &AppState, form: &NoteForm, editing: Option<i64>) -> Result<i64, AppError> {
    let slug = slugify(form.slug.as_deref().unwrap_or(""));
    let status = if form.status.as_deref() == Some("published") { "published" } else { "draft" };
    let published = valid_date(form.published_at.as_deref().unwrap_or("")).unwrap_or_else(util::today);
    let tags = form.tags.clone().unwrap_or_default();
    let license = form.license.clone().unwrap_or_default();
    let guest = form.guest_author_id.filter(|v| *v > 0);

    let id = if let Some(id) = editing {
        sqlx::query(
            "UPDATE notes SET slug = ?, status = ?, published_at = ?, updated_at = ?, guest_author_id = ?, license = ?, tags = ? WHERE id = ?",
        )
        .bind(&slug)
        .bind(status)
        .bind(&published)
        .bind(util::now_utc())
        .bind(guest)
        .bind(if license.is_empty() { None } else { Some(license.clone()) })
        .bind(&tags)
        .bind(id)
        .execute(&st.db)
        .await?;
        id
    } else {
        let res = sqlx::query(
            "INSERT INTO notes (slug, lang, status, published_at, created_at, updated_at, guest_author_id, license, tags)
             VALUES (?, 'zh', ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&slug)
        .bind(status)
        .bind(&published)
        .bind(util::now_utc())
        .bind(util::now_utc())
        .bind(guest)
        .bind(if license.is_empty() { None } else { Some(license.clone()) })
        .bind(&tags)
        .execute(&st.db)
        .await?;
        res.last_insert_rowid()
    };

    for (lang, title, summary, body) in [
        ("zh", form.zh_title.clone().unwrap_or_default(), form.zh_summary.clone().unwrap_or_default(), form.zh_body.clone().unwrap_or_default()),
        ("en", form.en_title.clone().unwrap_or_default(), form.en_summary.clone().unwrap_or_default(), form.en_body.clone().unwrap_or_default()),
    ] {
        if title.trim().is_empty() {
            sqlx::query("DELETE FROM note_i18n WHERE note_id = ? AND lang = ?")
                .bind(id)
                .bind(lang)
                .execute(&st.db)
                .await?;
        } else {
            sqlx::query(
                "INSERT INTO note_i18n (note_id, lang, title, summary, body) VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(note_id, lang) DO UPDATE SET title = excluded.title, summary = excluded.summary, body = excluded.body",
            )
            .bind(id)
            .bind(lang)
            .bind(title.trim())
            .bind(summary.trim())
            .bind(body)
            .execute(&st.db)
            .await?;
        }
    }
    Ok(id)
}

async fn note_form_page(st: &AppState, session: &SessionRow, note_id: Option<i64>) -> Result<Response, AppError> {
    let authors: Vec<GuestAuthorRow> = sqlx::query_as(
        "SELECT id, name, bio, contact_email, status, created_at FROM guest_authors ORDER BY id DESC",
    )
    .fetch_all(&st.db)
    .await?;

    let (id, slug, status, published, tags, license, guest, zh_title, zh_summary, zh_body, en_title, en_summary, en_body) =
        if let Some(id) = note_id {
            let base: (String, String, Option<String>, String, Option<String>, Option<i64>) = sqlx::query_as(
                "SELECT slug, status, published_at, tags, license, guest_author_id FROM notes WHERE id = ?",
            )
            .bind(id)
            .fetch_one(&st.db)
            .await?;
            let zh: Option<(String, String, String)> = sqlx::query_as(
                "SELECT title, summary, body FROM note_i18n WHERE note_id = ? AND lang = 'zh'",
            )
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
            let en: Option<(String, String, String)> = sqlx::query_as(
                "SELECT title, summary, body FROM note_i18n WHERE note_id = ? AND lang = 'en'",
            )
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
            (Some(id), base.0, base.1, base.2, base.3, base.4, base.5,
             zh.as_ref().map(|v| v.0.clone()).unwrap_or_default(),
             zh.as_ref().map(|v| v.1.clone()).unwrap_or_default(),
             zh.as_ref().map(|v| v.2.clone()).unwrap_or_default(),
             en.as_ref().map(|v| v.0.clone()).unwrap_or_default(),
             en.as_ref().map(|v| v.1.clone()).unwrap_or_default(),
             en.as_ref().map(|v| v.2.clone()).unwrap_or_default())
        } else {
            (None, String::new(), "draft".to_string(), None, String::new(), None, None,
             String::new(), String::new(), String::new(), String::new(), String::new(), String::new())
        };

    let title = if id.is_some() { "编辑笔记" } else { "新建笔记" };
    let body = html! {
        h1 { (title) }
        form method="post" action={(if let Some(id) = id { format!("/admin/notes/{}", id) } else { "/admin/notes".to_string() })} class="admin-form" {
            input type="hidden" name="csrf" value=(session.csrf);
            div class="form-row" {
                div class="field" { label { "slug（URL）" } input name="slug" value=(slug) required maxlength="80"; }
                div class="field" { label { "状态" }
                    select name="status" {
                        option value="draft" selected[status != "published"] { "草稿" }
                        option value="published" selected[status == "published"] { "发布" }
                    }
                }
                div class="field" { label { "发布日期" } input name="published_at" type="date" value=(published.clone().unwrap_or_default()); }
            }
            div class="form-row" {
                div class="field" { label { "标签（逗号分隔）" } input name="tags" value=(tags); }
                div class="field" { label { "作者（读者文章）" }
                    select name="guest_author_id" {
                        option value="0" selected[guest.is_none()] { "站长本人" }
                        @for a in &authors {
                            option value=(a.id) selected[guest == Some(a.id)] { (a.name) }
                        }
                    }
                }
            }
            div class="field" { label { "授权声明（读者文章）" } input name="license" value=(license.unwrap_or_default()); }
            h2 { "中文版本" }
            div class="field" { label { "标题" } input name="zh_title" value=(zh_title); }
            div class="field" { label { "摘要" } textarea name="zh_summary" rows="2" { (zh_summary) } }
            div class="field" { label { "正文（Markdown）" } textarea name="zh_body" rows="18" { (zh_body) } }
            h2 { "English version" }
            div class="field" { label { "Title" } input name="en_title" value=(en_title); }
            div class="field" { label { "Summary" } textarea name="en_summary" rows="2" { (en_summary) } }
            div class="field" { label { "Body (Markdown)" } textarea name="en_body" rows="18" { (en_body) } }
            p class="hint" { "任一语言标题留空，即表示该语言暂无译文（前台会显式提示并回退到原文）。" }
            p { button class="btn" type="submit" { "保存" } }
        }
        @if let Some(id) = id {
            form method="post" action={(format!("/admin/notes/{}/delete", id))} class="danger-zone" {
                input type="hidden" name="csrf" value=(session.csrf);
                button class="btn btn-danger" type="submit" { "删除这篇笔记" }
            }
        }
    };
    Ok(Html(admin_page(&st, title, "notes", &session.csrf, body).await.into_string()).into_response())
}

pub async fn note_new(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    note_form_page(&st, &sess, None).await
}

pub async fn note_edit(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    note_form_page(&st, &sess, Some(id)).await
}

pub async fn note_create(
    State(st): State<AppState>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<NoteForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let id = save_note(&st, &form, None).await?;
    audit(&st.db, "note_create", &format!("note:{}", id), form.zh_title.as_deref().unwrap_or(""), &util::client_ip(&headers)).await;
    Ok(Redirect::to("/admin/notes").into_response())
}

pub async fn note_update(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<NoteForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    save_note(&st, &form, Some(id)).await?;
    audit(&st.db, "note_update", &format!("note:{}", id), form.zh_title.as_deref().unwrap_or(""), &util::client_ip(&headers)).await;
    Ok(Redirect::to("/admin/notes").into_response())
}

#[derive(Deserialize)]
pub struct IdForm {
    pub id: Option<i64>,
    pub csrf: String,
}

pub async fn note_delete(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<IdForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    sqlx::query("DELETE FROM notes WHERE id = ?").bind(id).execute(&st.db).await?;
    audit(&st.db, "note_delete", &format!("note:{}", id), "", &util::client_ip(&headers)).await;
    Ok(Redirect::to("/admin/notes").into_response())
}

pub async fn subscribers(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let rows: Vec<SubscriberRow> = sqlx::query_as(
        "SELECT id, email, lang, source, status, created_at, confirmed_at, unsubscribed_at
         FROM subscribers WHERE status != 'deleted' ORDER BY id DESC LIMIT 500",
    )
    .fetch_all(&st.db)
    .await?;
    let body = html! {
        h1 { "订阅者" }
        p class="hint" { "邮箱仅在此页可见；公开页面与公开 JSON 不包含任何邮箱字段。删除为硬删除，不可恢复。" }
        table class="table" {
            thead { tr { th { "ID" } th { "邮箱" } th { "语言" } th { "来源" } th { "状态" } th { "创建时间" } th { "确认时间" } th { "" } } }
            tbody {
                @for s in &rows {
                    tr {
                        td { (s.id) }
                        td { (s.email) }
                        td { (s.lang) }
                        td { (s.source) }
                        td { (match s.status.as_str() { "confirmed" => "已确认", "pending" => "待确认", _ => "已退订" }) }
                        td { (s.created_at) }
                        td { (s.confirmed_at.clone().unwrap_or_default()) }
                        td {
                            form method="post" action={(format!("/admin/subscribers/{}/delete", s.id))} class="inline-form" {
                                input type="hidden" name="csrf" value=(sess.csrf);
                                button class="btn btn-danger btn-sm" type="submit" { "删除" }
                            }
                        }
                    }
                }
            }
        }
    };
    Ok(Html(admin_page(&st, "订阅者", "subscribers", &sess.csrf, body).await.into_string()).into_response())
}

pub async fn subscriber_delete(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<CsrfForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    sqlx::query("DELETE FROM subscribers WHERE id = ?").bind(id).execute(&st.db).await?;
    audit(&st.db, "subscriber_delete", &format!("subscriber:{}", id), "", &util::client_ip(&headers)).await;
    Ok(Redirect::to("/admin/subscribers").into_response())
}

#[derive(Deserialize)]
pub struct AuthorForm {
    pub name: Option<String>,
    pub bio: Option<String>,
    pub contact_email: Option<String>,
    pub csrf: String,
}

pub async fn authors(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let rows: Vec<GuestAuthorRow> = sqlx::query_as(
        "SELECT id, name, bio, contact_email, status, created_at FROM guest_authors ORDER BY id DESC",
    )
    .fetch_all(&st.db)
    .await?;
    let body = html! {
        h1 { "受邀作者" }
        div class="split-2" {
            div {
                table class="table" {
                    thead { tr { th { "ID" } th { "姓名" } th { "简介" } th { "状态" } th { "" } } }
                    tbody {
                        @for a in &rows {
                            tr {
                                td { (a.id) }
                                td { (a.name) }
                                td { (a.bio) }
                                td { (a.status) }
                                td {
                                    form method="post" action={(format!("/admin/authors/{}/invite", a.id))} class="inline-form" {
                                        input type="hidden" name="csrf" value=(sess.csrf);
                                        button class="btn btn-sm" type="submit" { "生成邀请码" }
                                    }
                                    form method="post" action="/admin/authors/status" class="inline-form" {
                                        input type="hidden" name="csrf" value=(sess.csrf);
                                        input type="hidden" name="id" value=(a.id);
                                        input type="hidden" name="status" value="active";
                                        button class="btn btn-quiet btn-sm" type="submit" { "设为有效" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div class="form-box" {
                h2 { "新增受邀作者" }
                form method="post" action="/admin/authors" {
                    input type="hidden" name="csrf" value=(sess.csrf);
                    div class="field" { label { "姓名" } input name="name" required maxlength="80"; }
                    div class="field" { label { "简介" } textarea name="bio" rows="4" { } }
                    div class="field" { label { "联系邮箱（私密）" } input name="contact_email" type="email" maxlength="254"; }
                    button class="btn" type="submit" { "添加" }
                }
            }
        }
    };
    Ok(Html(admin_page(&st, "受邀作者", "authors", &sess.csrf, body).await.into_string()).into_response())
}

pub async fn author_create(
    State(st): State<AppState>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<AuthorForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let name = form.name.unwrap_or_default().trim().to_string();
    if name.is_empty() {
        return Ok(Redirect::to("/admin/authors").into_response());
    }
    let res = sqlx::query(
        "INSERT INTO guest_authors (name, bio, contact_email, status, created_at) VALUES (?, ?, ?, 'invited', ?)",
    )
    .bind(&name)
    .bind(form.bio.unwrap_or_default().trim())
    .bind(form.contact_email.filter(|e| !e.trim().is_empty()))
    .bind(util::now_utc())
    .execute(&st.db)
    .await?;
    audit(&st.db, "author_create", &format!("author:{}", res.last_insert_rowid()), &name, &util::client_ip(&headers)).await;
    Ok(Redirect::to("/admin/authors").into_response())
}

#[derive(Deserialize)]
pub struct StatusForm {
    pub id: Option<i64>,
    pub status: Option<String>,
    pub csrf: String,
}

pub async fn author_status(
    State(st): State<AppState>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<StatusForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    if let Some(id) = form.id {
        let status = match form.status.as_deref() {
            Some("active") => "active",
            Some("revoked") => "revoked",
            _ => "invited",
        };
        sqlx::query("UPDATE guest_authors SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&st.db)
            .await?;
        audit(&st.db, "author_status", &format!("author:{}", id), status, &util::client_ip(&headers)).await;
    }
    Ok(Redirect::to("/admin/authors").into_response())
}

pub async fn author_invite(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<CsrfForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let author: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT name, contact_email FROM guest_authors WHERE id = ?")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    let Some((name, email)) = author else {
        return Ok(Redirect::to("/admin/authors").into_response());
    };
    let code = util::token(16);
    sqlx::query("INSERT INTO invites (guest_author_id, code_hash, expires_at, created_at) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(sha256_hex(&code))
        .bind(util::plus_hours(72))
        .bind(util::now_utc())
        .execute(&st.db)
        .await?;
    audit(&st.db, "author_invite", &format!("author:{}", id), &name, &util::client_ip(&headers)).await;

    let link = format!("{}/zh/contribute?invite={}", st.cfg.site_url.trim_end_matches('/'), code);
    if let Some(email) = &email {
        let body = format!(
            "{} 你好：\n\n邀请你为 wayhug.com 写一篇原创文章。这个链接 72 小时内有效：\n{}\n\n如果链接过期，回信给我，我再生成一个。\n\n—\n王元会 · wayhug.com\n",
            name, link
        );
        st.mailer
            .send("author_invite", email, "wayhug.com 受邀投稿", &body)
            .await;
    }

    let body = html! {
        h1 { "邀请码已生成" }
        p { "作者：" (name) }
        p class="hint" { "邀请码只显示这一次；关闭页面后需重新生成（旧码仍有效，直到使用或过期）。" }
        div class="form-box" {
            p { strong { "投稿链接" } }
            code class="code-block" { (link) }
            p { strong { "邀请码" } }
            code class="code-block" { (code) }
        }
        @if email.is_none() {
            p class="notice warn" { "该作者没有联系邮箱，请手动把链接发给他。" }
        } @else if !st.mailer.smtp_enabled {
            p class="notice warn" { "SMTP 未配置，邀请邮件只写入发件箱，请手动发送上面的链接。" }
        }
        p { a class="btn" href="/admin/authors" { "返回" } }
    };
    Ok(Html(admin_page(&st, "邀请码", "authors", &sess.csrf, body).await.into_string()).into_response())
}

pub async fn submissions(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let rows: Vec<SubmissionRow> = sqlx::query_as(
        "SELECT id, guest_author_id, name, email, lang, title, body, license, status, created_at, note_id
         FROM submissions ORDER BY id DESC LIMIT 200",
    )
    .fetch_all(&st.db)
    .await?;
    let body = html! {
        h1 { "投稿审核" }
        p class="hint" { "通过后会按投稿语言建立一篇笔记，署名作者并附授权声明。" }
        @for sub in &rows {
            article class="submission" {
                header {
                    h2 { (sub.title) }
                    p class="note-meta" {
                        (sub.name) " · " (sub.lang) " · " (sub.created_at)
                        " · 状态：" (sub.status)
                        @if let Some(id) = sub.note_id { " · 笔记 #" (id) }
                    }
                }
                details {
                    summary { "查看正文" }
                    (ui::prose(&crate::md::render(&sub.body)))
                }
                @if sub.status == "pending" {
                    div class="row-actions" {
                        form method="post" action={(format!("/admin/submissions/{}/approve", sub.id))} class="inline-form" {
                            input type="hidden" name="csrf" value=(sess.csrf);
                            button class="btn btn-sm" type="submit" { "通过并发布" }
                        }
                        form method="post" action={(format!("/admin/submissions/{}/reject", sub.id))} class="inline-form" {
                            input type="hidden" name="csrf" value=(sess.csrf);
                            button class="btn btn-quiet btn-sm" type="submit" { "驳回" }
                        }
                    }
                }
            }
        }
    };
    Ok(Html(admin_page(&st, "投稿审核", "submissions", &sess.csrf, body).await.into_string()).into_response())
}

pub async fn submission_approve(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<CsrfForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let sub: Option<(String, String, String, Option<i64>, String)> = sqlx::query_as(
        "SELECT title, body, lang, guest_author_id, license FROM submissions WHERE id = ? AND status = 'pending'",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?;
    let Some((title, body, lang, author_id, license)) = sub else {
        return Ok(Redirect::to("/admin/submissions").into_response());
    };
    let mut slug = slugify(&title);
    if slug.starts_with("note-") {
        slug = format!("guest-{}", id);
    }
    let outcome = sqlx::query(
        "INSERT INTO notes (slug, lang, status, published_at, created_at, updated_at, guest_author_id, license, tags)
         VALUES (?, ?, 'published', ?, ?, ?, ?, ?, '读者文章')",
    )
    .bind(&slug)
    .bind(&lang)
    .bind(util::today())
    .bind(util::now_utc())
    .bind(util::now_utc())
    .bind(author_id)
    .bind(&license)
    .execute(&st.db)
    .await;

    let note_id = match outcome {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            tracing::warn!("approve submission failed: {}", e);
            return Ok(Redirect::to("/admin/submissions").into_response());
        }
    };
    sqlx::query("INSERT INTO note_i18n (note_id, lang, title, summary, body) VALUES (?, ?, ?, '', ?)")
        .bind(note_id)
        .bind(&lang)
        .bind(&title)
        .bind(&body)
        .execute(&st.db)
        .await?;
    sqlx::query("UPDATE submissions SET status = 'approved', reviewed_at = ?, note_id = ? WHERE id = ?")
        .bind(util::now_utc())
        .bind(note_id)
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(&st.db, "submission_approve", &format!("submission:{}", id), &title, &util::client_ip(&headers)).await;
    Ok(Redirect::to("/admin/submissions").into_response())
}

pub async fn submission_reject(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<CsrfForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    sqlx::query("UPDATE submissions SET status = 'rejected', reviewed_at = ? WHERE id = ?")
        .bind(util::now_utc())
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(&st.db, "submission_reject", &format!("submission:{}", id), "", &util::client_ip(&headers)).await;
    Ok(Redirect::to("/admin/submissions").into_response())
}

async fn product_form_page(st: &AppState, session: &SessionRow, id: Option<i64>) -> Result<Response, AppError> {
    let (slug, price, currency, stock, zh_name, zh_desc, zh_revenue, en_name, en_desc, en_revenue) =
        if let Some(id) = id {
            let base: (String, i64, String, String) = sqlx::query_as(
                "SELECT slug, price_cents, currency, stock FROM products WHERE id = ?",
            )
            .bind(id)
            .fetch_one(&st.db)
            .await?;
            let zh: Option<(String, String, String)> = sqlx::query_as(
                "SELECT name, description, revenue_note FROM product_i18n WHERE product_id = ? AND lang = 'zh'",
            )
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
            let en: Option<(String, String, String)> = sqlx::query_as(
                "SELECT name, description, revenue_note FROM product_i18n WHERE product_id = ? AND lang = 'en'",
            )
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
            (base.0, base.1, base.2, base.3,
             zh.as_ref().map(|v| v.0.clone()).unwrap_or_default(),
             zh.as_ref().map(|v| v.1.clone()).unwrap_or_default(),
             zh.as_ref().map(|v| v.2.clone()).unwrap_or_default(),
             en.as_ref().map(|v| v.0.clone()).unwrap_or_default(),
             en.as_ref().map(|v| v.1.clone()).unwrap_or_default(),
             en.as_ref().map(|v| v.2.clone()).unwrap_or_default())
        } else {
            (String::new(), 0, "CNY".to_string(), "in_stock".to_string(),
             String::new(), String::new(), String::new(), String::new(), String::new(), String::new())
        };
    let title = if id.is_some() { "编辑文创" } else { "新建文创" };
    let body = html! {
        h1 { (title) }
        form method="post" action={(if let Some(id) = id { format!("/admin/products/{}", id) } else { "/admin/products".to_string() })} class="admin-form" {
            input type="hidden" name="csrf" value=(session.csrf);
            div class="form-row" {
                div class="field" { label { "slug" } input name="slug" value=(slug) required; }
                div class="field" { label { "价格（元）" } input name="price" value={(format!("{}.{:02}", price / 100, price % 100))} inputmode="decimal" required; }
                div class="field" { label { "币种" } input name="currency" value=(currency); }
                div class="field" { label { "库存状态" }
                    select name="stock" {
                        option value="in_stock" selected[stock == "in_stock"] { "有货" }
                        option value="preorder" selected[stock == "preorder"] { "预售" }
                        option value="sold_out" selected[stock == "sold_out"] { "售罄" }
                    }
                }
            }
            h2 { "中文" }
            div class="field" { label { "名称" } input name="zh_name" value=(zh_name) required; }
            div class="field" { label { "说明" } textarea name="zh_desc" rows="4" { (zh_desc) } }
            div class="field" { label { "收益用途" } textarea name="zh_revenue" rows="3" { (zh_revenue) } }
            h2 { "English" }
            div class="field" { label { "Name" } input name="en_name" value=(en_name); }
            div class="field" { label { "Description" } textarea name="en_desc" rows="4" { (en_desc) } }
            div class="field" { label { "Where the money goes" } textarea name="en_revenue" rows="3" { (en_revenue) } }
            p { button class="btn" type="submit" { "保存" } }
        }
    };
    Ok(Html(admin_page(&st, title, "products", &session.csrf, body).await.into_string()).into_response())
}

#[derive(Deserialize)]
pub struct ProductForm {
    pub slug: Option<String>,
    pub price: Option<String>,
    pub currency: Option<String>,
    pub stock: Option<String>,
    pub zh_name: Option<String>,
    pub zh_desc: Option<String>,
    pub zh_revenue: Option<String>,
    pub en_name: Option<String>,
    pub en_desc: Option<String>,
    pub en_revenue: Option<String>,
    pub csrf: String,
}

fn parse_price(raw: &str) -> i64 {
    let v: f64 = raw.trim().parse().unwrap_or(0.0);
    (v.max(0.0) * 100.0).round() as i64
}

async fn save_product(st: &AppState, form: &ProductForm, id: Option<i64>) -> Result<i64, AppError> {
    let slug = slugify(form.slug.as_deref().unwrap_or(""));
    let price = parse_price(form.price.as_deref().unwrap_or("0"));
    let currency = form.currency.clone().unwrap_or_else(|| "CNY".to_string());
    let stock = match form.stock.as_deref() {
        Some("preorder") => "preorder",
        Some("sold_out") => "sold_out",
        _ => "in_stock",
    };
    let id = if let Some(id) = id {
        sqlx::query("UPDATE products SET slug = ?, price_cents = ?, currency = ?, stock = ?, updated_at = ? WHERE id = ?")
            .bind(&slug).bind(price).bind(&currency).bind(stock).bind(util::now_utc()).bind(id)
            .execute(&st.db).await?;
        id
    } else {
        let res = sqlx::query("INSERT INTO products (slug, price_cents, currency, stock, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&slug).bind(price).bind(&currency).bind(stock).bind(util::now_utc()).bind(util::now_utc())
            .execute(&st.db).await?;
        res.last_insert_rowid()
    };
    for (lang, name, desc, revenue) in [
        ("zh", form.zh_name.clone().unwrap_or_default(), form.zh_desc.clone().unwrap_or_default(), form.zh_revenue.clone().unwrap_or_default()),
        ("en", form.en_name.clone().unwrap_or_default(), form.en_desc.clone().unwrap_or_default(), form.en_revenue.clone().unwrap_or_default()),
    ] {
        if name.trim().is_empty() {
            sqlx::query("DELETE FROM product_i18n WHERE product_id = ? AND lang = ?")
                .bind(id).bind(lang).execute(&st.db).await?;
        } else {
            sqlx::query(
                "INSERT INTO product_i18n (product_id, lang, name, description, revenue_note) VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(product_id, lang) DO UPDATE SET name = excluded.name, description = excluded.description, revenue_note = excluded.revenue_note",
            )
            .bind(id).bind(lang).bind(name.trim()).bind(desc).bind(revenue)
            .execute(&st.db).await?;
        }
    }
    Ok(id)
}

pub async fn products(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let rows: Vec<(i64, String, i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT p.id, p.slug, p.price_cents, p.currency, p.stock,
                (SELECT name FROM product_i18n WHERE product_id = p.id AND lang = 'zh')
         FROM products p ORDER BY p.id",
    )
    .fetch_all(&st.db)
    .await?;
    let body = html! {
        div class="admin-head" {
            h1 { "文创" }
            a class="btn" href="/admin/products/new" { "新建文创" }
        }
        table class="table" {
            thead { tr { th { "ID" } th { "slug" } th { "名称" } th { "价格" } th { "库存" } th { "" } } }
            tbody {
                @for (id, slug, price, currency, stock, name) in &rows {
                    tr {
                        td { (id) }
                        td { (slug) }
                        td { (name.clone().unwrap_or_default()) }
                        td { (currency) " " (format!("{}.{:02}", price / 100, price % 100)) }
                        td { (stock) }
                        td { a href={(format!("/admin/products/{}", id))} { "编辑" } }
                    }
                }
            }
        }
    };
    Ok(Html(admin_page(&st, "文创", "products", &sess.csrf, body).await.into_string()).into_response())
}

pub async fn product_new(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    product_form_page(&st, &sess, None).await
}

pub async fn product_edit(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    product_form_page(&st, &sess, Some(id)).await
}

pub async fn product_create(
    State(st): State<AppState>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<ProductForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let id = save_product(&st, &form, None).await?;
    audit(&st.db, "product_create", &format!("product:{}", id), form.zh_name.as_deref().unwrap_or(""), &util::client_ip(&headers)).await;
    Ok(Redirect::to("/admin/products").into_response())
}

pub async fn product_update(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<ProductForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    save_product(&st, &form, Some(id)).await?;
    audit(&st.db, "product_update", &format!("product:{}", id), form.zh_name.as_deref().unwrap_or(""), &util::client_ip(&headers)).await;
    Ok(Redirect::to("/admin/products").into_response())
}

pub async fn audit_page(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let rows: Vec<AuditRow> = sqlx::query_as(
        "SELECT at, actor, action, target, detail, ip FROM audit_log ORDER BY id DESC LIMIT 200",
    )
    .fetch_all(&st.db)
    .await?;
    let body = html! {
        h1 { "审计日志" }
        table class="table" {
            thead { tr { th { "时间" } th { "操作者" } th { "动作" } th { "对象" } th { "详情" } th { "IP" } } }
            tbody {
                @for a in &rows {
                    tr {
                        td { (a.at) } td { (a.actor) } td { (a.action) }
                        td { (a.target.clone().unwrap_or_default()) }
                        td { (a.detail.clone().unwrap_or_default()) }
                        td { (a.ip.clone().unwrap_or_default()) }
                    }
                }
            }
        }
    };
    Ok(Html(admin_page(&st, "审计日志", "audit", &sess.csrf, body).await.into_string()).into_response())
}

pub async fn outbox(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let rows: Vec<MailRow> = sqlx::query_as(
        "SELECT id, to_email, subject, kind, status, error, created_at FROM mail_outbox ORDER BY id DESC LIMIT 100",
    )
    .fetch_all(&st.db)
    .await?;
    let body = html! {
        h1 { "发件箱" }
        p class="hint" {
            @if st.mailer.smtp_enabled { "SMTP 已配置。这里记录所有投递结果（sent / failed）。" }
            @else { "SMTP 未接入：邮件不会真实发出，状态为 logged，全文可在下行查看。接入后在 .env 配置 SMTP_* 并重启即可。" }
        }
        table class="table" {
            thead { tr { th { "时间" } th { "收件人" } th { "主题" } th { "类型" } th { "状态" } th { "错误" } th { "" } } }
            tbody {
                @for m in &rows {
                    tr {
                        td { (m.created_at) }
                        td { (m.to_email) }
                        td { (m.subject) }
                        td { (m.kind) }
                        td { (m.status) }
                        td { (m.error.clone().unwrap_or_default()) }
                        td { a href={(format!("/admin/outbox/{}", m.id))} { "查看" } }
                    }
                }
            }
        }
    };
    Ok(Html(admin_page(&st, "发件箱", "outbox", &sess.csrf, body).await.into_string()).into_response())
}

pub async fn outbox_view(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let row: Option<(String, String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT to_email, subject, body, status, error, created_at FROM mail_outbox WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?;
    let Some((to, subject, body_text, status, error, created_at)) = row else {
        return Ok(Redirect::to("/admin/outbox").into_response());
    };
    let body = html! {
        h1 { "邮件 #" (id) }
        p class="note-meta" { (created_at) " · " (status) }
        p { "收件人：" (to) }
        p { "主题：" (subject) }
        @if let Some(e) = error { p class="notice warn" { (e) } }
        pre class="mail-body" { (body_text) }
        p { a class="btn" href="/admin/outbox" { "返回" } }
    };
    Ok(Html(admin_page(&st, "邮件", "outbox", &sess.csrf, body).await.into_string()).into_response())
}
