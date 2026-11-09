use axum::extract::{Path, Query, RawForm, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use maud::{Markup, PreEscaped, html};
use serde::Deserialize;

use crate::AppState;
use crate::admin::{admin_page, audit, guard};
use crate::models::SubmissionRow2;
use crate::public::not_found_page;
use crate::reader_notes::preview_render;
use crate::util::{self, AppError};

#[derive(Deserialize, Default)]
pub struct Filters {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub book: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub id: Option<i64>,
}

impl Filters {
    fn status(&self) -> String {
        match self.status.as_deref() {
            Some("pending") => "pending".to_string(),
            Some("accepted") => "accepted".to_string(),
            Some("rejected") => "rejected".to_string(),
            Some("all") => "all".to_string(),
            _ => "pending".to_string(),
        }
    }

    fn q(&self) -> String {
        self.q.as_deref().unwrap_or("").trim().to_string()
    }

    fn book(&self) -> String {
        self.book.as_deref().unwrap_or("").trim().to_string()
    }

    fn from(&self) -> String {
        self.from.as_deref().unwrap_or("").trim().to_string()
    }

    fn to(&self) -> String {
        self.to.as_deref().unwrap_or("").trim().to_string()
    }
}

const REASONS: [(&str, &str); 5] = [
    ("", "选择退回原因…"),
    ("incomplete", "内容不完整"),
    ("off_topic", "与读书无关"),
    ("spam", "疑似广告"),
    ("other", "其他"),
];

fn reason_label(code: &str) -> &'static str {
    REASONS
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| *l)
        .unwrap_or("其他")
}

async fn load_queue(st: &AppState, f: &Filters) -> Result<Vec<SubmissionRow2>, sqlx::Error> {
    sqlx::query_as::<_, SubmissionRow2>(
        "SELECT s.* FROM note_submissions s
         WHERE (?1 = 'all' OR s.status = ?1)
           AND (?2 = '' OR s.title LIKE '%'||?2||'%' OR s.body LIKE '%'||?2||'%'
                OR s.book_title LIKE '%'||?2||'%' OR s.question LIKE '%'||?2||'%')
           AND (?3 = '' OR s.book_title LIKE '%'||?3||'%')
           AND (?4 = '' OR date(s.created_at) >= date(?4))
           AND (?5 = '' OR date(s.created_at) <= date(?5))
         ORDER BY s.created_at DESC, s.id DESC
         LIMIT 100",
    )
    .bind(f.status())
    .bind(f.q())
    .bind(f.book())
    .bind(f.from())
    .bind(f.to())
    .fetch_all(&st.db)
    .await
}

async fn load_one(st: &AppState, id: i64) -> Result<Option<SubmissionRow2>, sqlx::Error> {
    sqlx::query_as::<_, SubmissionRow2>("SELECT * FROM note_submissions WHERE id = ?")
        .bind(id)
        .fetch_optional(&st.db)
        .await
}

async fn counts(st: &AppState) -> (i64, i64, i64, i64) {
    let mut pending = 0;
    let mut accepted = 0;
    let mut rejected = 0;
    if let Ok(rows) = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*) FROM note_submissions GROUP BY status",
    )
    .fetch_all(&st.db)
    .await
    {
        for (k, n) in rows {
            match k.as_str() {
                "pending" => pending = n,
                "accepted" => accepted = n,
                "rejected" => rejected = n,
                _ => {}
            }
        }
    }
    (pending, accepted, rejected, pending + accepted + rejected)
}

fn status_url(status: &str) -> String {
    format!("/admin/review?status={}", status)
}

fn item_url(status: &str, id: i64) -> String {
    format!("/admin/review?status={}&id={}", status, id)
}

fn status_tabs(current: &str, c: (i64, i64, i64, i64)) -> Markup {
    html! {
        nav class="status-tabs" aria-label="按状态筛选" {
            a class=(if current == "pending" { "active" } else { "" }) href=(status_url("pending")) {
                "待审 " (c.0)
            }
            a class=(if current == "accepted" { "active" } else { "" }) href=(status_url("accepted")) {
                "已采纳 " (c.1)
            }
            a class=(if current == "rejected" { "active" } else { "" }) href=(status_url("rejected")) {
                "已退回 " (c.2)
            }
            a class=(if current == "all" { "active" } else { "" }) href=(status_url("all")) {
                "全部 " (c.3)
            }
        }
    }
}

fn meta_line(s: &SubmissionRow2) -> Markup {
    html! {
        p class="ri-meta" {
            (s.created_at)
            " · "
            (if s.book_title.is_empty() { "未填书名" } else { &s.book_title })
            " · "
            (s.words()) " 字"
            @if !s.ip_region.is_empty() {
                " · " (s.ip_region)
            }
        }
    }
}

fn queue_list(rows: &[SubmissionRow2], selected: Option<i64>, status: &str, csrf: &str) -> Markup {
    if rows.is_empty() {
        return html! {
            div class="queue-empty" {
                strong { "今天没有待审" }
                p { "新提交会出现在这里。也可以切换上方状态查看已处理记录。" }
            }
        };
    }
    html! {
        fieldset class="review-list" {
            legend class="sr-only" { "待审队列" }
            @for s in rows {
                div class={"review-item" (if selected == Some(s.id) { " active" } else { "" })} {
                    input class="review-check" type="checkbox" name="ids" value=(s.id) aria-label="选择该条以批量退回";
                    a class="review-link" href=(item_url(status, s.id)) {
                        div class="ri-title" { (s.title) }
                        (meta_line(s))
                    }
                }
            }
        }
        div class="batch-bar" {
            label for="batch-reason" { "批量退回" }
            select id="batch-reason" name="batch_reason" {
                @for (val, label) in REASONS {
                    option value=(val) { (label) }
                }
            }
            button class="btn btn-sm btn-danger" type="submit" { "退回选中" }
            input type="hidden" name="csrf" value=(csrf);
        }
    }
}

fn read_only_panel(s: &SubmissionRow2, _csrf: &str) -> Markup {
    let is_accepted = s.status == "accepted";
    html! {
        section class="review-panel" {
            h1 { (s.title) }
            div class="shortcut-bar" {
                span { "回执 " strong { (s.receipt) } }
                span { "状态 " span class="status-pill" { (s.status_label()) } }
                @if let Some(at) = &s.reviewed_at { span { "处理于 " (at) } }
                @if let Some(reason) = &s.reject_reason {
                    span { "原因 " (reason_label(reason)) }
                }
            }
            @if let Some(note) = &s.review_note {
                @if !note.is_empty() { p class="hint" { "审核意见：" (note) } }
            }
            (submission_meta(s))
            div class="prose" { (PreEscaped(crate::md::render(&s.body))) }
            @if !s.question.is_empty() {
                div class="rn-question" {
                    h2 { "留下的问题" }
                    p { (s.question) }
                }
            }
            @if is_accepted {
                @if let Some(note_id) = s.note_id {
                    p { a class="btn btn-primary" href={(format!("/admin/notes/{}", note_id))} { "去编辑草稿（补译文与配图）" } }
                }
            }
            details {
                summary { "原始提交快照（只读）" }
                pre class="code-block" { (s.original_json) }
            }
            p { a class="btn" href=(format!("/admin/review?status={}", s.status)) { "返回队列" } }
            p class="hint" { "已处理的记录不再改动，仅保留追溯。" }
        }
    }
}

fn submission_meta(s: &SubmissionRow2) -> Markup {
    let weather = match s.weather.as_str() {
        "sunny" => "晴",
        "cloudy" => "多云",
        "overcast" => "阴",
        "rain" => "雨",
        "snow" => "雪",
        _ => "",
    };
    let date = s.occurred_at.replace('T', " ");
    html! {
        dl class="reader-note-meta" {
            @if !date.is_empty() { div { dt { "日期" } dd { (date) } } }
            @if let Some(w) = s.weekday { div { dt { "星期" } dd { (crate::reader_notes::weekday_label(crate::i18n::Lang::Zh, w)) } } }
            @if !s.place.is_empty() { div { dt { "地点" } dd { (s.place) } } }
            @if !weather.is_empty() || !s.temp_c.is_empty() {
                div { dt { "天气" } dd { (weather) (if s.temp_c.is_empty() { String::new() } else { format!(" · {}", s.temp_c) }) } }
            }
            @if !s.book_title.is_empty() { div { dt { "书名" } dd { (s.book_title) } } }
            @if !s.isbn.is_empty() { div { dt { "ISBN" } dd { (s.isbn) } } }
            div { dt { "回执" } dd { (s.receipt) } }
            div { dt { "来源" } dd { (if s.ip_region.is_empty() { "未知".to_string() } else { s.ip_region.clone() }) } }
            div { dt { "字数" } dd { (s.words()) } }
        }
    }
}

async fn audit_rows(st: &AppState, id: i64) -> Vec<(String, String, String)> {
    sqlx::query_as::<_, (String, String, String)>(
        "SELECT at, action, COALESCE(detail, '') FROM audit_log
         WHERE target = ? ORDER BY at DESC, id DESC LIMIT 20",
    )
    .bind(format!("note_submission:{}", id))
    .fetch_all(&st.db)
    .await
    .unwrap_or_default()
}

async fn pending_edit_panel(st: &AppState, s: &SubmissionRow2, csrf: &str) -> Markup {
    let preview = preview_render(&s.body, &s.question).await;
    let history = audit_rows(st, s.id).await;
    html! {
        section class="review-panel" {
            h1 { (s.title) }
            div class="shortcut-bar" {
                span { span class="kbd" { "A" } " 采纳" }
                span { span class="kbd" { "R" } " 退回" }
                span { span class="kbd" { "J" } " 下一条" }
                span { span class="kbd" { "K" } " 上一条" }
                span { span class="kbd" { "Ctrl/⌘ + Enter" } " 采纳并下一条" }
            }
            (submission_meta(s))

            form method="post" action=(format!("/admin/review/{}/decide", s.id)) id="rv-form" {
                input type="hidden" name="csrf" value=(csrf);

                div class="field" {
                    label for="rv-title" { "标题（可改，改动写审计）" }
                    input id="rv-title" name="title" type="text" maxlength="120" value=(s.title);
                }
                div class="rn-grid two" {
                    div class="field" {
                        label for="rv-book" { "书名" }
                        input id="rv-book" name="book_title" type="text" maxlength="160" value=(s.book_title);
                    }
                    div class="field" {
                        label for="rv-links" { "相关链接（每行一条）" }
                        input id="rv-links" name="links" type="text" value=(s.links);
                    }
                }
                div class="field" {
                    label for="rv-body" { "正文（Markdown）" }
                    textarea id="rv-body" name="body" rows="14" maxlength="20000" { (s.body) }
                }
                div class="field" {
                    label for="rv-question" { "思考 / 问题" }
                    input id="rv-question" name="question" type="text" maxlength="280" value=(s.question);
                }

                div class="field" {
                    label { "实时预览（与前台同一渲染器）" }
                    div class="review-preview" id="rv-preview" { (PreEscaped(preview)) }
                }

                div class="form-actions" {
                    button class="btn" type="submit" name="action" value="save" { "保存修改" }
                    button class="btn btn-primary" type="submit" name="action" value="accept" id="rv-accept" { "采纳并生成草稿（A）" }
                    span class="review-reject" {
                        select name="reject_reason" id="rv-reason" aria-label="退回原因" {
                            @for (val, label) in REASONS {
                                option value=(val) { (label) }
                            }
                        }
                        button class="btn btn-sm btn-danger" type="submit" name="action" value="reject" id="rv-reject" { "退回（R）" }
                    }
                }
                div class="field" {
                    label for="rv-note" { "审核意见（退回时可选）" }
                    input id="rv-note" name="review_note" type="text" maxlength="200";
                }
            }

            p class="hint" {
                "J/K 在队列中移动；" span class="kbd" { "Ctrl/⌘+Enter" } " 采纳并下一条。输入框内不拦截字母键。"
            }

            @if !history.is_empty() {
                details {
                    summary { "这条记录的审计（最近 20 条）" }
                    ul class="plain-list" {
                        @for (at, action, detail) in &history {
                            li { (at) " · " (action) @if !detail.is_empty() { " · " (detail) } }
                        }
                    }
                }
            }
        }
    }
}

pub async fn workbench(
    State(st): State<AppState>,
    headers: HeaderMap,
    Query(f): Query<Filters>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let status = f.status();
    let rows = load_queue(&st, &f).await?;

    let selected = match f.id {
        Some(id) if rows.iter().any(|r| r.id == id) => Some(id),
        Some(id) => load_one(&st, id).await?.map(|r| r.id),
        None => rows.first().map(|r| r.id),
    };
    let current = match selected {
        Some(id) => load_one(&st, id).await?,
        None => None,
    };

    let idx = current
        .as_ref()
        .and_then(|c| rows.iter().position(|r| r.id == c.id));
    let prev_id = idx.and_then(|i| if i > 0 { rows.get(i - 1) } else { None }).map(|r| r.id);
    let next_id = idx
        .and_then(|i| rows.get(i + 1))
        .map(|r| r.id);

    let panel = match &current {
        Some(s) => {
            let nav = html! {
                div class="batch-bar" {
                    @if let Some(id) = prev_id {
                        a id="rv-prev" class="btn btn-sm" href=(item_url(&status, id)) { "上一条 (K)" }
                    } @else {
                        span id="rv-prev" {}
                    }
                    @if let Some(id) = next_id {
                        a id="rv-next" class="btn btn-sm" href=(item_url(&status, id)) { "下一条 (J)" }
                    } @else {
                        span id="rv-next" {}
                    }
                }
            };
            let body = if s.status == "pending" {
                pending_edit_panel(&st, s, &sess.csrf).await
            } else {
                read_only_panel(s, &sess.csrf)
            };
            html! { (nav) (body) }
        }
        None => html! {
            section class="review-panel queue-empty" {
                strong { "今天没有待审" }
                p { "新提交会出现在这里；也可以切换状态查看已采纳或已退回的记录。" }
            }
        },
    };

    let c = counts(&st).await;
    let content = html! {
        div class="review-bar" {
            h1 { "读者笔记审阅" @if c.0 > 0 { span class="queue-badge" { (c.0) } } }
            span class="hint" { "默认进入待审队列；采纳只生成草稿，发布前前台不可见。" }
        }
        (status_tabs(&status, c))
        form method="get" action="/admin/review" class="filters" {
            input type="hidden" name="status" value=(status);
            div { label for="f-q" { "关键词" } input id="f-q" name="q" type="search" value=(f.q()) placeholder="标题 / 正文 / 书名"; }
            div { label for="f-book" { "书名" } input id="f-book" name="book" type="search" value=(f.book()); }
            div { label for="f-from" { "起始日期" } input id="f-from" name="from" type="date" value=(f.from()); }
            div { label for="f-to" { "结束日期" } input id="f-to" name="to" type="date" value=(f.to()); }
            div { button class="btn btn-sm" type="submit" { "筛选" } }
        }
        div class="review-layout" {
            form method="post" action="/admin/review/batch-reject" {
                (queue_list(&rows, selected, &status, &sess.csrf))
            }
            div { (panel) }
        }
        script src="/static/review.js" defer {}
    };

    Ok(Html(admin_page(&st, "读者笔记审阅", "review", &sess.csrf, content).await.into_string())
        .into_response())
}

#[derive(Deserialize)]
pub struct DecideForm {
    pub csrf: String,
    pub action: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub book_title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub links: Option<String>,
    #[serde(default)]
    pub reject_reason: Option<String>,
    #[serde(default)]
    pub review_note: Option<String>,
}

fn short(s: &str, max: usize) -> String {
    let t: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        format!("{}…", t)
    } else {
        t
    }
}

async fn apply_edits(
    st: &AppState,
    s: &mut SubmissionRow2,
    f: &DecideForm,
    ip: &str,
) -> Result<Vec<String>, AppError> {
    let mut diffs = Vec::new();
    if let Some(title) = &f.title {
        let t = title.trim();
        if !t.is_empty() && t != s.title {
            diffs.push(format!("title: 「{}」→「{}」", short(&s.title, 60), short(t, 60)));
            s.title = t.to_string();
        }
    }
    if let Some(book) = &f.book_title {
        let b = book.trim();
        if b != s.book_title {
            diffs.push(format!("book: 「{}」→「{}」", short(&s.book_title, 60), short(b, 60)));
            s.book_title = b.to_string();
        }
    }
    if let Some(body) = &f.body {
        if body != &s.body {
            diffs.push(format!(
                "body: {} 字 → {} 字",
                s.body.chars().count(),
                body.chars().count()
            ));
            s.body = body.clone();
        }
    }
    if let Some(q) = &f.question {
        let q = q.trim();
        if q != s.question {
            diffs.push(format!("question: 「{}」→「{}」", short(&s.question, 40), short(q, 40)));
            s.question = q.to_string();
        }
    }
    if let Some(links) = &f.links {
        let l = links.trim();
        if l != s.links {
            diffs.push(format!("links: 更新为 {}", short(l, 80)));
            s.links = l.to_string();
        }
    }
    if !diffs.is_empty() {
        sqlx::query(
            "UPDATE note_submissions SET title = ?, book_title = ?, body = ?, question = ?,
             links = ?, edited_at = ?, edited_by = 'admin' WHERE id = ? AND status = 'pending'",
        )
        .bind(&s.title)
        .bind(&s.book_title)
        .bind(&s.body)
        .bind(&s.question)
        .bind(&s.links)
        .bind(util::now_utc())
        .bind(s.id)
        .execute(&st.db)
        .await?;
        audit(
            &st.db,
            "note_submission_edit",
            &format!("note_submission:{}", s.id),
            &diffs.join("；"),
            ip,
        )
        .await;
    }
    Ok(diffs)
}

fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !last_dash && !out.is_empty() {
                out.push('-');
                last_dash = true;
            }
        }
    }
    out.trim_matches('-').to_string()
}

async fn unique_slug(st: &AppState, base: &str) -> Result<String, sqlx::Error> {
    let mut slug = base.to_string();
    let mut n = 2;
    loop {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE slug = ?")
            .bind(&slug)
            .fetch_one(&st.db)
            .await?;
        if exists == 0 {
            return Ok(slug);
        }
        slug = format!("{}-{}", base, n);
        n += 1;
    }
}

async fn accept(
    st: &AppState,
    s: &SubmissionRow2,
    ip: &str,
) -> Result<i64, AppError> {
    let base = {
        let slug = slugify(&s.title);
        if slug.is_empty() {
            format!("reader-note-{}", s.id)
        } else {
            slug
        }
    };
    let slug = unique_slug(st, &base).await?;
    let summary = {
        let p = crate::md::plain(&s.body, 120);
        if p.is_empty() {
            crate::md::plain(&s.question, 120)
        } else {
            p
        }
    };
    let now = util::now_utc();

    let mut tx = st.db.begin().await?;
    let note_id = sqlx::query(
        "INSERT INTO notes (slug, lang, status, published_at, created_at, updated_at, guest_author_id, license, tags)
         VALUES (?, ?, 'draft', NULL, ?, ?, NULL, NULL, '读者笔记')",
    )
    .bind(&slug)
    .bind(&s.lang)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    sqlx::query(
        "INSERT INTO note_i18n (note_id, lang, title, summary, body) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(note_id)
    .bind(&s.lang)
    .bind(&s.title)
    .bind(&summary)
    .bind(&s.body)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO reader_note_meta
         (note_id, submission_id, occurred_at, weekday, place, weather, temp_c,
          book_title, isbn, question, color, flag, links, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(note_id)
    .bind(s.id)
    .bind(&s.occurred_at)
    .bind(s.weekday)
    .bind(&s.place)
    .bind(&s.weather)
    .bind(&s.temp_c)
    .bind(&s.book_title)
    .bind(&s.isbn)
    .bind(&s.question)
    .bind(&s.color)
    .bind(&s.flag)
    .bind(&s.links)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE note_submissions SET status = 'accepted', note_id = ?, reviewer = 'admin',
         reviewed_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(note_id)
    .bind(&now)
    .bind(s.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    audit(
        &st.db,
        "note_submission_accept",
        &format!("note_submission:{}", s.id),
        &format!("生成笔记草稿 #{} slug={}", note_id, slug),
        ip,
    )
    .await;

    Ok(note_id)
}

async fn reject(
    st: &AppState,
    s: &SubmissionRow2,
    code: &str,
    note: &str,
    ip: &str,
) -> Result<(), AppError> {
    let code = if code.is_empty() { "other" } else { code };
    sqlx::query(
        "UPDATE note_submissions SET status = 'rejected', reject_reason = ?, review_note = ?,
         reviewer = 'admin', reviewed_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(code)
    .bind(note)
    .bind(util::now_utc())
    .bind(s.id)
    .execute(&st.db)
    .await?;
    audit(
        &st.db,
        "note_submission_reject",
        &format!("note_submission:{}", s.id),
        &format!("原因 {}；{}", code, note),
        ip,
    )
    .await;
    Ok(())
}

async fn next_pending(st: &AppState) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM note_submissions WHERE status = 'pending'
         ORDER BY created_at DESC, id DESC LIMIT 1",
    )
    .fetch_optional(&st.db)
    .await
}

pub async fn decide(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    axum::Form(f): axum::Form<DecideForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &f.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let Some(mut s) = load_one(&st, id).await? else {
        return Ok(not_found_page(crate::i18n::Lang::Zh, &st.cfg.site_url));
    };
    if s.status != "pending" {
        return Ok(Redirect::to(&format!("/admin/review?status={}&id={}", s.status, s.id))
            .into_response());
    }
    let ip = util::client_ip(&headers);
    let diffs = apply_edits(&st, &mut s, &f, &ip).await?;

    match f.action.as_str() {
        "accept" => {
            let note_id = accept(&st, &s, &ip).await?;
            let _ = diffs;
            match next_pending(&st).await? {
                Some(next) => Ok(Redirect::to(&format!(
                    "/admin/review?status=pending&id={}&accepted={}",
                    next, note_id
                ))
                .into_response()),
                None => Ok(Redirect::to(&format!(
                    "/admin/review?status=pending&accepted={}",
                    note_id
                ))
                .into_response()),
            }
        }
        "reject" => {
            reject(
                &st,
                &s,
                f.reject_reason.as_deref().unwrap_or(""),
                f.review_note.as_deref().unwrap_or(""),
                &ip,
            )
            .await?;
            match next_pending(&st).await? {
                Some(next) => Ok(
                    Redirect::to(&format!("/admin/review?status=pending&id={}", next))
                        .into_response(),
                ),
                None => Ok(Redirect::to("/admin/review?status=pending").into_response()),
            }
        }
        _ => Ok(Redirect::to(&format!("/admin/review?status=pending&id={}", s.id)).into_response()),
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        out.push((h * 16 + l) as u8);
                        i += 3;
                    }
                    _ => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// 浏览器对多选复选框会重复提交同名 ids，serde_urlencoded 无法反序列化成 Vec，
// 因此这里直接读原始表单体解析。
pub async fn batch_reject(
    State(st): State<AppState>,
    headers: HeaderMap,
    RawForm(raw): RawForm,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let body = String::from_utf8_lossy(&raw);
    let mut csrf = String::new();
    let mut code = String::new();
    let mut ids = Vec::new();
    for pair in body.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let (k, v) = (percent_decode(k), percent_decode(v));
        match k.as_str() {
            "csrf" => csrf = v,
            "batch_reason" => code = v,
            "ids" => ids.push(v),
            _ => {}
        }
    }
    if !util::ct_eq(&sess.csrf, &csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    let ip = util::client_ip(&headers);
    let code = if code.is_empty() { "other" } else { code.as_str() };
    let mut n = 0;
    for raw in &ids {
        if let Ok(id) = raw.parse::<i64>() {
            if let Some(s) = load_one(&st, id).await? {
                if s.status == "pending" {
                    reject(&st, &s, code, "批量退回", &ip).await?;
                    n += 1;
                }
            }
        }
    }
    Ok(Redirect::to(&format!(
        "/admin/review?status=rejected&batch={}",
        n
    ))
    .into_response())
}

#[derive(Deserialize)]
pub struct PreviewForm {
    pub csrf: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub question: String,
}

pub async fn preview(
    State(st): State<AppState>,
    headers: HeaderMap,
    axum::Form(f): axum::Form<PreviewForm>,
) -> Result<Response, AppError> {
    let sess = match guard(&st, &headers).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    if !util::ct_eq(&sess.csrf, &f.csrf) {
        return Ok((StatusCode::FORBIDDEN, "CSRF check failed").into_response());
    }
    Ok(Html(preview_render(&f.body, &f.question).await).into_response())
}
