use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use maud::html;
use serde::Deserialize;

use crate::AppState;
use crate::i18n::Lang;
use crate::public::{csrf_for, csrf_ok, page_response};
use crate::ui;
use crate::util::{self, AppError, sha256_hex};

#[derive(Deserialize)]
pub struct SentQuery {
    pub sent: Option<String>,
}

#[derive(Deserialize)]
pub struct SubForm {
    pub email: String,
    pub lang: Option<String>,
    pub consent: Option<String>,
    pub csrf: String,
    pub website: Option<String>,
}

#[derive(Deserialize)]
pub struct TokenQuery {
    pub token: Option<String>,
}

#[derive(Deserialize)]
pub struct TokenForm {
    pub token: String,
    pub csrf: String,
}

fn valid_email(raw: &str) -> bool {
    let e = raw.trim();
    if e.len() < 6 || e.len() > 254 || e.chars().any(char::is_whitespace) {
        return false;
    }
    let parts: Vec<&str> = e.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return false;
    }
    let domain = parts[1];
    domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn confirm_mail(lang: Lang, site: &str, token: &str, manage: &str) -> (String, String) {
    let confirm_url = format!("{}/{}/subscribe/confirm?token={}", site, lang.code(), token);
    let manage_url = format!("{}/{}/subscribe/manage?token={}", site, lang.code(), manage);
    match lang {
        Lang::Zh => (
            "确认订阅 wayhug.com".to_string(),
            format!(
                "你好：\n\n你（或使用这个邮箱的人）在 wayhug.com 提交了订阅。\n\
                 点击下面的链接完成确认（48 小时内有效）：\n\n{}\n\n\
                 确认后，有新笔记时我会写一封短信给你；每年一月寄一份年度简账。\n\
                 如果这不是你做的，忽略这封邮件即可，不会再有后续邮件。\n\n\
                 退订或删除邮箱：{}\n\n\
                 —\n王元会 · wayhug.com\n",
                confirm_url, manage_url
            ),
        ),
        Lang::En => (
            "Confirm your subscription to wayhug.com".to_string(),
            format!(
                "Hello,\n\nSomeone using this address asked to subscribe at wayhug.com.\n\
                 Click the link below to confirm (valid for 48 hours):\n\n{}\n\n\
                 After confirming, I will write a short letter when a new note appears, and send a plain annual account each January.\n\
                 If this was not you, ignore this message; nothing further will be sent.\n\n\
                 Unsubscribe or delete this address: {}\n\n\
                 —\nWang Yuanhui · wayhug.com\n",
                confirm_url, manage_url
            ),
        ),
    }
}

fn manage_mail(lang: Lang, site: &str, manage: &str) -> (String, String) {
    let manage_url = format!("{}/{}/subscribe/manage?token={}", site, lang.code(), manage);
    match lang {
        Lang::Zh => (
            "wayhug.com 订阅管理".to_string(),
            format!(
                "你已经在 wayhug.com 订阅。\n\n退订或删除邮箱：{}\n\n—\n王元会 · wayhug.com\n",
                manage_url
            ),
        ),
        Lang::En => (
            "Your wayhug.com subscription".to_string(),
            format!(
                "You are subscribed at wayhug.com.\n\nUnsubscribe or delete this address: {}\n\n—\nWang Yuanhui · wayhug.com\n",
                manage_url
            ),
        ),
    }
}

pub async fn subscribe_page(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    Query(q): Query<SentQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(crate::public::not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let (csrf, new) = csrf_for(&headers);
    let action = format!("/{}/subscribe", lang.code());
    let content = html! {
        section class="narrow" {
            h1 { (s.subscribe_title) }
            p class="lead" { (s.subscribe_intro) }
            @if q.sent.is_some() {
                div class="notice ok" {
                    h2 { (s.sent_title) }
                    p { (s.sent_body) }
                    p class="hint" { (s.sent_hint) }
                }
            }
            div class="form-box" {
                (ui::subscribe_form(lang, &csrf, &action))
            }
            @if !st.mailer.smtp_enabled {
                p class="hint dev-note" { "（当前部署未接入 SMTP，确认邮件写入服务器发件箱与日志，由站长手动确认。）" }
            }
        }
    };
    let path = format!("/{}/subscribe", lang.code());
    Ok(page_response(
        ui::page(lang, &path, s.subscribe_title, s.subscribe_intro, "subscribe", &st.cfg.site_url, content),
        new,
        &st,
    ))
}

pub async fn subscribe_submit(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<SubForm>,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(crate::public::not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let target = format!("/{}/subscribe?sent=1", lang.code());

    if !csrf_ok(&headers, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    if form.website.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false) {
        return Ok(Redirect::to(&target).into_response());
    }
    let ip = util::client_ip(&headers);
    if !st.limiter.allow(&format!("sub:{}", ip), 5, Duration::from_secs(3600)) {
        return Ok((StatusCode::TOO_MANY_REQUESTS, "too many requests").into_response());
    }
    if form.consent.as_deref() != Some("1") || !valid_email(&form.email) {
        return Ok(Redirect::to(&format!("/{}/subscribe?sent=0", lang.code())).into_response());
    }

    let email = form.email.trim().to_lowercase();
    let chosen = form
        .lang
        .as_deref()
        .and_then(Lang::from_seg)
        .unwrap_or(lang);
    let confirm = util::token(32);
    let manage = util::token(32);
    let confirm_hash = sha256_hex(&confirm);
    let manage_hash = sha256_hex(&manage);
    let expires = util::plus_hours(48);

    let existing: Option<(i64, String)> =
        sqlx::query_as("SELECT id, status FROM subscribers WHERE email = ?")
            .bind(&email)
            .fetch_optional(&st.db)
            .await?;

    let kind;
    if let Some((id, status)) = existing {
        if status == "confirmed" {
            sqlx::query(
                "UPDATE subscribers SET manage_token_hash = ?, lang = ?, last_mail_at = ? WHERE id = ?",
            )
            .bind(&manage_hash)
            .bind(chosen.code())
            .bind(util::now_utc())
            .bind(id)
            .execute(&st.db)
            .await?;
            kind = "manage";
        } else {
            sqlx::query(
                "UPDATE subscribers
                 SET status = 'pending', lang = ?, confirm_token_hash = ?, confirm_expires_at = ?,
                     manage_token_hash = ?, last_mail_at = ?, unsubscribed_at = NULL
                 WHERE id = ?",
            )
            .bind(chosen.code())
            .bind(&confirm_hash)
            .bind(&expires)
            .bind(&manage_hash)
            .bind(util::now_utc())
            .bind(id)
            .execute(&st.db)
            .await?;
            kind = "confirm";
        }
    } else {
        sqlx::query(
            "INSERT INTO subscribers
             (email, lang, source, status, created_at, confirm_token_hash, confirm_expires_at, manage_token_hash, last_mail_at, ip_hash)
             VALUES (?, ?, ?, 'pending', ?, ?, ?, ?, ?, ?)",
        )
        .bind(&email)
        .bind(chosen.code())
        .bind(format!("web:{}", lang.code()))
        .bind(util::now_utc())
        .bind(&confirm_hash)
        .bind(&expires)
        .bind(&manage_hash)
        .bind(util::now_utc())
        .bind(util::ip_hash(&headers))
        .execute(&st.db)
        .await?;
        kind = "confirm";
    }

    if kind == "confirm" {
        let (subject, body) = confirm_mail(chosen, st.cfg.site_url.trim_end_matches('/'), &confirm, &manage);
        st.mailer.send("subscribe_confirm", &email, &subject, &body).await;
    } else {
        let (subject, body) = manage_mail(chosen, st.cfg.site_url.trim_end_matches('/'), &manage);
        st.mailer.send("subscribe_manage", &email, &subject, &body).await;
    }

    Ok(Redirect::to(&target).into_response())
}

pub async fn confirm(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    Query(q): Query<TokenQuery>,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(crate::public::not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let Some(token) = q.token.filter(|t| !t.is_empty()) else {
        return Ok(invalid_page(&st, lang, s.invalid_token_title, s.invalid_token_body));
    };
    let hash = sha256_hex(&token);
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, confirm_expires_at FROM subscribers
         WHERE confirm_token_hash = ? AND status = 'pending'",
    )
    .bind(&hash)
    .fetch_optional(&st.db)
    .await?;

    let Some((id, expires)) = row else {
        return Ok(invalid_page(&st, lang, s.invalid_token_title, s.invalid_token_body));
    };
    if util::is_expired(&expires) {
        return Ok(invalid_page(&st, lang, s.invalid_token_title, s.invalid_token_body));
    }

    sqlx::query(
        "UPDATE subscribers
         SET status = 'confirmed', confirmed_at = ?, confirm_token_hash = NULL, confirm_expires_at = NULL
         WHERE id = ?",
    )
    .bind(util::now_utc())
    .bind(id)
    .execute(&st.db)
    .await?;

    Ok(Redirect::to(&format!("/{}/subscribe/done", lang.code())).into_response())
}

pub async fn done(
    State(st): State<AppState>,
    Path(lang): Path<String>,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(crate::public::not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let content = html! {
        section class="narrow" {
            h1 { (s.confirmed_title) }
            p class="lead" { (s.confirmed_body) }
            p { a class="btn" href={(format!("/{}/notes", lang.code()))} { (s.nav_notes) } }
            p class="hint" { (s.manage_hint) }
        }
    };
    let path = format!("/{}/subscribe/done", lang.code());
    Ok(page_response(
        ui::page(lang, &path, s.confirmed_title, s.confirmed_body, "subscribe", &st.cfg.site_url, content),
        None,
        &st,
    ))
}

async fn find_by_manage(db: &sqlx::SqlitePool, token: &str) -> Result<Option<(i64, String, String)>, sqlx::Error> {
    let hash = sha256_hex(token);
    sqlx::query_as("SELECT id, status, lang FROM subscribers WHERE manage_token_hash = ?")
        .bind(hash)
        .fetch_optional(db)
        .await
}

pub async fn manage(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(crate::public::not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let Some(token) = q.token.clone().filter(|t| !t.is_empty()) else {
        return Ok(invalid_page(&st, lang, s.manage_title, s.manage_token_invalid));
    };
    let Some((_id, status, _)) = find_by_manage(&st.db, &token).await? else {
        return Ok(invalid_page(&st, lang, s.manage_title, s.manage_token_invalid));
    };
    let (csrf, new) = csrf_for(&headers);
    let status_text = match status.as_str() {
        "confirmed" => s.status_confirmed,
        "unsubscribed" | "deleted" => s.status_unsubscribed,
        _ => s.status_pending,
    };
    let content = html! {
        section class="narrow" {
            h1 { (s.manage_title) }
            p {
                span class="label" { (s.manage_status_label) "：" }
                strong { (status_text) }
            }
            p class="hint" { (s.manage_hint) }
            form method="post" action={(format!("/{}/subscribe/unsubscribe", lang.code()))} class="inline-form" {
                input type="hidden" name="token" value=(token);
                input type="hidden" name="csrf" value=(csrf);
                button class="btn btn-quiet" type="submit" { (s.manage_unsubscribe) }
            }
            form method="post" action={(format!("/{}/subscribe/delete", lang.code()))} class="inline-form" {
                input type="hidden" name="token" value=(token);
                input type="hidden" name="csrf" value=(csrf);
                button class="btn btn-danger" type="submit" { (s.manage_delete) }
                p class="hint" { (s.manage_delete_warn) }
            }
        }
    };
    let path = format!("/{}/subscribe/manage", lang.code());
    Ok(page_response(
        ui::page(lang, &path, s.manage_title, s.manage_hint, "subscribe", &st.cfg.site_url, content),
        new,
        &st,
    ))
}

pub async fn unsubscribe(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<TokenForm>,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(crate::public::not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    if !csrf_ok(&headers, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let Some((id, _, _)) = find_by_manage(&st.db, &form.token).await? else {
        return Ok(invalid_page(&st, lang, s.manage_title, s.manage_token_invalid));
    };
    sqlx::query(
        "UPDATE subscribers SET status = 'unsubscribed', unsubscribed_at = ?, confirm_token_hash = NULL, confirm_expires_at = NULL WHERE id = ?",
    )
    .bind(util::now_utc())
    .bind(id)
    .execute(&st.db)
    .await?;
    Ok(invalid_page(&st, lang, s.manage_title, s.manage_unsubscribed_ok))
}

pub async fn delete_data(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<TokenForm>,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(crate::public::not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    if !csrf_ok(&headers, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let Some((id, _, _)) = find_by_manage(&st.db, &form.token).await? else {
        return Ok(invalid_page(&st, lang, s.manage_title, s.manage_token_invalid));
    };
    sqlx::query(
        "UPDATE subscribers
         SET email = ?, status = 'deleted', unsubscribed_at = ?, confirm_token_hash = NULL,
             confirm_expires_at = NULL, manage_token_hash = NULL, ip_hash = NULL
         WHERE id = ?",
    )
    .bind(format!("deleted+{}@wayhug.invalid", id))
    .bind(util::now_utc())
    .bind(id)
    .execute(&st.db)
    .await?;
    Ok(invalid_page(&st, lang, s.manage_title, s.manage_deleted_ok))
}

fn invalid_page(st: &AppState, lang: Lang, title: &str, body: &str) -> Response {
    let content = html! {
        section class="narrow" {
            h1 { (title) }
            p { (body) }
            p { a class="btn" href={(format!("/{}/subscribe", lang.code()))} { (lang.s().nav_subscribe) } }
        }
    };
    let path = format!("/{}/subscribe", lang.code());
    page_response(
        ui::page(lang, &path, title, body, "subscribe", &st.cfg.site_url, content),
        None,
        st,
    )
}

// ---------- 受邀投稿 ----------

#[derive(Deserialize)]
pub struct ContributeQuery {
    pub invite: Option<String>,
}

#[derive(Deserialize)]
pub struct ContributeForm {
    pub invite: String,
    pub title: String,
    pub body: String,
    pub agree: Option<String>,
    pub csrf: String,
}

struct Invite {
    id: i64,
    author_id: i64,
    author_name: String,
    author_email: Option<String>,
}

async fn check_invite(db: &sqlx::SqlitePool, code: &str) -> Result<Option<Invite>, sqlx::Error> {
    let hash = sha256_hex(code.trim());
    let row: Option<(i64, i64, String, Option<String>, String, Option<String>)> = sqlx::query_as(
        "SELECT i.id, a.id, a.name, a.contact_email, i.expires_at, i.used_at
         FROM invites i JOIN guest_authors a ON a.id = i.guest_author_id
         WHERE i.code_hash = ? LIMIT 1",
    )
    .bind(hash)
    .fetch_optional(db)
    .await?;
    Ok(row.and_then(|(id, author_id, name, email, expires, used)| {
        if used.is_some() || util::is_expired(&expires) {
            None
        } else {
            Some(Invite {
                id,
                author_id,
                author_name: name,
                author_email: email,
            })
        }
    }))
}

pub async fn contribute_page(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    Query(q): Query<ContributeQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(crate::public::not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let (csrf, new) = csrf_for(&headers);
    let code = q.invite.unwrap_or_default();
    let invite = if code.is_empty() {
        None
    } else {
        check_invite(&st.db, &code).await?
    };
    let content = html! {
        section class="narrow" {
            h1 { (s.contribute_title) }
            p class="lead" { (s.contribute_intro) }
            @if !code.is_empty() && invite.is_none() {
                (ui::notice("warn", s.contribute_invalid))
            }
            form class="form-box" method="post" action={(format!("/{}/contribute", lang.code()))} {
                input type="hidden" name="csrf" value=(csrf);
                div class="field" {
                    label for="invite" { (s.field_invite) }
                    input id="invite" name="invite" required value=(code) autocomplete="off";
                }
                @if let Some(inv) = &invite {
                    p class="hint" { "受邀作者 / Invited author：" (inv.author_name) }
                }
                div class="field" {
                    label for="title" { (s.field_title) }
                    input id="title" name="title" required maxlength="160";
                }
                div class="field" {
                    label for="body" { (s.field_body) }
                    textarea id="body" name="body" required rows="14" maxlength="50000";
                }
                label class="check" {
                    input type="checkbox" name="agree" value="1" required;
                    span { (s.label_agree_license) }
                }
                button class="btn" type="submit" { (s.submit_contribution) }
            }
        }
    };
    let path = format!("/{}/contribute", lang.code());
    Ok(page_response(
        ui::page(lang, &path, s.contribute_title, s.contribute_intro, "", &st.cfg.site_url, content),
        new,
        &st,
    ))
}

pub async fn contribute_submit(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<ContributeForm>,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(crate::public::not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    if !csrf_ok(&headers, &form.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    if form.agree.as_deref() != Some("1")
        || form.title.trim().is_empty()
        || form.body.trim().len() < 50
        || form.body.len() > 50000
    {
        return Ok(invalid_page(&st, lang, s.contribute_title, s.contribute_invalid));
    }
    let Some(invite) = check_invite(&st.db, &form.invite).await? else {
        return Ok(invalid_page(&st, lang, s.contribute_title, s.contribute_invalid));
    };

    sqlx::query(
        "INSERT INTO submissions (guest_author_id, name, email, lang, title, body, license, status, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?)",
    )
    .bind(invite.author_id)
    .bind(&invite.author_name)
    .bind(&invite.author_email)
    .bind(lang.code())
    .bind(form.title.trim())
    .bind(&form.body)
    .bind(s.label_agree_license)
    .bind(util::now_utc())
    .execute(&st.db)
    .await?;
    sqlx::query("UPDATE invites SET used_at = ? WHERE id = ?")
        .bind(util::now_utc())
        .bind(invite.id)
        .execute(&st.db)
        .await?;

    let excerpt: String = form.body.chars().take(1200).collect();
    let mail_body = format!(
        "作者：{}\n语言：{}\n标题：{}\n\n{}\n\n（全文请到后台投稿审核页查看）\n",
        invite.author_name,
        lang.code(),
        form.title.trim(),
        excerpt
    );
    st.mailer
        .send(
            "submission_notice",
            &st.cfg.contact_email,
            &format!("[wayhug] 新的受邀投稿：{}", form.title.trim()),
            &mail_body,
        )
        .await;

    let content = html! {
        section class="narrow" {
            h1 { (s.contribute_done_title) }
            p class="lead" { (s.contribute_done_body) }
            p { a class="btn" href={(format!("/{}/", lang.code()))} { (s.back_home) } }
        }
    };
    let path = format!("/{}/contribute", lang.code());
    Ok(page_response(
        ui::page(lang, &path, s.contribute_done_title, s.contribute_done_body, "", &st.cfg.site_url, content),
        None,
        &st,
    ))
}

#[allow(dead_code)]
fn unused_imports_marker(_: Option<header::HeaderName>) -> Html<String> {
    Html(String::new())
}
