use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, Utc};
use maud::{Markup, html};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::AppState;
use crate::i18n::Lang;
use crate::md;
use crate::models::{NoteCard, ReaderMeta};
use crate::public::{csrf_for, csrf_ok, load_notes, not_found_page, page_response};
use crate::ui;
use crate::util::{self, AppError};

pub const MAX_TITLE: usize = 120;
pub const MAX_BODY: usize = 20000;
pub const MAX_PLACE: usize = 80;
pub const MAX_BOOK: usize = 160;
pub const MAX_QUESTION: usize = 280;
pub const MAX_LINKS: usize = 5;
pub const RATE_MAX: usize = 3;
pub const RATE_WINDOW_SECS: u64 = 3600;

#[derive(Default, Clone)]
pub struct Values {
    pub occurred_at: String,
    pub weekday: String,
    pub place: String,
    pub weather: String,
    pub temp_c: String,
    pub book_title: String,
    pub isbn: String,
    pub title: String,
    pub body: String,
    pub question: String,
    pub color: String,
    pub flag: String,
    pub links: String,
}

#[derive(Deserialize, Default)]
pub struct SubmitForm {
    #[serde(default)]
    pub csrf: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub quiz_a: String,
    #[serde(default)]
    pub quiz_b: String,
    #[serde(default)]
    pub quiz: String,
    #[serde(default)]
    pub occurred_at: String,
    #[serde(default)]
    pub weekday: String,
    #[serde(default)]
    pub place: String,
    #[serde(default)]
    pub weather: String,
    #[serde(default)]
    pub temp_c: String,
    #[serde(default)]
    pub book_title: String,
    #[serde(default)]
    pub isbn: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub flag: String,
    #[serde(default)]
    pub links: String,
}

#[derive(Default)]
pub struct Errors {
    pub title: bool,
    pub isbn: bool,
    pub links: bool,
    pub quiz: bool,
    pub body: bool,
    pub long: bool,
    pub rate: bool,
    pub bad_link: Vec<String>,
}

impl Errors {
    pub fn any(&self) -> bool {
        self.title || self.isbn || self.links || self.quiz || self.body || self.long || self.rate
    }
}

pub fn china_now() -> chrono::DateTime<Utc> {
    Utc::now() + ChronoDuration::hours(8)
}

fn default_datetime() -> String {
    china_now().format("%Y-%m-%dT%H:%M").to_string()
}

fn default_weekday(dt: &str) -> String {
    let d = dt.get(0..10).unwrap_or("");
    NaiveDate::parse_from_str(d, "%Y-%m-%d")
        .map(|d| d.weekday().num_days_from_monday().to_string())
        .unwrap_or_else(|_| {
            china_now()
                .date_naive()
                .weekday()
                .num_days_from_monday()
                .to_string()
        })
}

pub fn weekday_label(lang: Lang, w: i64) -> &'static str {
    lang.s().rn_weekdays[(w.clamp(0, 6)) as usize]
}

pub fn weather_label(lang: Lang, code: &str) -> &'static str {
    let s = lang.s();
    match code {
        "sunny" => s.rn_weather_sunny,
        "cloudy" => s.rn_weather_cloudy,
        "overcast" => s.rn_weather_overcast,
        "rain" => s.rn_weather_rain,
        "snow" => s.rn_weather_snow,
        _ => "",
    }
}

pub fn color_label(lang: Lang, code: &str) -> &'static str {
    let s = lang.s();
    match code {
        "apricot" => s.rn_color_apricot,
        "lilac" => s.rn_color_lilac,
        "indigo" => s.rn_color_indigo,
        "gray" => s.rn_color_gray,
        _ => "",
    }
}

pub fn flag_label(_lang: Lang, code: &str) -> &'static str {
    match code {
        "star" => "★",
        "pennant" => "⚑",
        "diamond" => "◆",
        "dot" => "●",
        _ => "",
    }
}

pub fn normalize_isbn(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x')
        .collect::<String>()
        .to_ascii_uppercase()
}

pub fn isbn_valid(raw: &str) -> bool {
    let s = normalize_isbn(raw);
    match s.len() {
        10 => {
            let mut sum = 0i32;
            for (i, c) in s.chars().enumerate() {
                let v = if c == 'X' {
                    if i != 9 {
                        return false;
                    }
                    10
                } else if let Some(d) = c.to_digit(10) {
                    d as i32
                } else {
                    return false;
                };
                sum += (10 - i as i32) * v;
            }
            sum % 11 == 0
        }
        13 => {
            if !s.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let sum: i32 = s
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    let d = c.to_digit(10).unwrap_or(0) as i32;
                    if i % 2 == 0 { d } else { d * 3 }
                })
                .sum();
            sum % 10 == 0
        }
        _ => false,
    }
}

/// 每行一条，只接受 http/https；返回 (合法链接, 非法行)。
pub fn parse_links(raw: &str) -> (Vec<String>, Vec<String>) {
    let mut ok = Vec::new();
    let mut bad = Vec::new();
    for line in raw.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let lower = l.to_ascii_lowercase();
        if (lower.starts_with("http://") || lower.starts_with("https://"))
            && !l.chars().any(char::is_whitespace)
        {
            ok.push(l.to_string());
        } else {
            bad.push(l.to_string());
        }
    }
    (ok, bad)
}

fn validate(f: &SubmitForm) -> (Values, Errors) {
    let mut e = Errors::default();
    let v = Values {
        occurred_at: f.occurred_at.trim().to_string(),
        weekday: f.weekday.trim().to_string(),
        place: truncate_chars(f.place.trim(), MAX_PLACE),
        weather: match f.weather.trim() {
            x @ ("sunny" | "cloudy" | "overcast" | "rain" | "snow") => x.to_string(),
            _ => String::new(),
        },
        temp_c: truncate_chars(f.temp_c.trim(), 8),
        book_title: truncate_chars(f.book_title.trim(), MAX_BOOK),
        isbn: normalize_isbn(&f.isbn),
        title: f.title.trim().to_string(),
        body: f.body.trim().to_string(),
        question: f.question.trim().replace('\n', " "),
        color: match f.color.trim() {
            x @ ("apricot" | "lilac" | "indigo" | "gray") => x.to_string(),
            _ => String::new(),
        },
        flag: match f.flag.trim() {
            x @ ("star" | "pennant" | "diamond" | "dot") => x.to_string(),
            _ => String::new(),
        },
        links: f.links.clone(),
    };

    if v.title.is_empty() || v.title.chars().count() > MAX_TITLE {
        e.title = true;
    }
    if v.body.chars().count() > MAX_BODY {
        e.body = true;
    }
    if !v.isbn.is_empty() && !isbn_valid(&v.isbn) {
        e.isbn = true;
    }
    if v.question.chars().count() > MAX_QUESTION {
        e.long = true;
    }
    let (ok, bad) = parse_links(&v.links);
    if !bad.is_empty() {
        e.bad_link = bad;
        e.links = true;
    }
    if ok.len() > MAX_LINKS {
        e.links = true;
    }
    (v, e)
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn quiz_numbers() -> (i32, i32) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (rng.gen_range(2..=9), rng.gen_range(2..=9))
}

fn quiz_ok(f: &SubmitForm) -> bool {
    let a = f.quiz_a.trim().parse::<i32>().ok();
    let b = f.quiz_b.trim().parse::<i32>().ok();
    let ans = f.quiz.trim().parse::<i32>().ok();
    matches!((a, b, ans), (Some(a), Some(b), Some(c)) if a + b == c)
}

pub async fn load_meta(db: &SqlitePool, note_id: i64) -> Result<Option<ReaderMeta>, sqlx::Error> {
    sqlx::query_as::<_, ReaderMeta>(
        "SELECT note_id, submission_id, occurred_at, weekday, place, weather, temp_c,
                book_title, isbn, question, color, flag, links, created_at
         FROM reader_note_meta WHERE note_id = ?",
    )
    .bind(note_id)
    .fetch_optional(db)
    .await
}

pub fn meta_from_submission(s: &crate::models::SubmissionRow2) -> ReaderMeta {
    ReaderMeta {
        note_id: s.note_id.unwrap_or(0),
        submission_id: s.id,
        occurred_at: s.occurred_at.clone(),
        weekday: s.weekday,
        place: s.place.clone(),
        weather: s.weather.clone(),
        temp_c: s.temp_c.clone(),
        book_title: s.book_title.clone(),
        isbn: s.isbn.clone(),
        question: s.question.clone(),
        color: s.color.clone(),
        flag: s.flag.clone(),
        links: s.links.clone(),
        created_at: s.created_at.clone(),
    }
}

pub fn meta_block(lang: Lang, meta: &ReaderMeta) -> Markup {
    let s = lang.s();
    let date = meta.occurred_at.replace('T', " ");
    let date = if date.len() > 16 { date[..16].to_string() } else { date };
    let weather = weather_label(lang, &meta.weather);
    let weather = if meta.temp_c.is_empty() {
        weather.to_string()
    } else if weather.is_empty() {
        meta.temp_c.clone()
    } else {
        format!("{} · {}", weather, meta.temp_c)
    };
    let color = color_label(lang, &meta.color);
    let flag = flag_label(lang, &meta.flag);
    let (link_ok, _) = parse_links(&meta.links);
    html! {
        dl class="reader-note-meta" {
            @if !date.is_empty() {
                div {
                    dt { (s.rn_date) }
                    dd { time datetime=(meta.occurred_at) { (date) } }
                }
            }
            @if let Some(w) = meta.weekday {
                div {
                    dt { (s.rn_weekday) }
                    dd { (weekday_label(lang, w)) }
                }
            }
            @if !meta.place.is_empty() {
                div { dt { (s.rn_place) } dd { (meta.place) } }
            }
            @if !weather.is_empty() {
                div { dt { (s.rn_weather) } dd { (weather) } }
            }
            @if !meta.book_title.is_empty() || !meta.isbn.is_empty() {
                div {
                    dt { (s.reader_note_book) }
                    dd {
                        (meta.book_title)
                        @if !meta.isbn.is_empty() {
                            span class="dot" { "·" } span { (s.reader_note_isbn) " " (meta.isbn) }
                        }
                    }
                }
            }
            @if !color.is_empty() || !flag.is_empty() {
                div {
                    dt { (s.reader_note_marks) }
                    dd class="rn-mark" {
                        @if !color.is_empty() {
                            span class={"rn-color-chip rn-color-" (meta.color)} aria-hidden="true" {}
                            span { (color) }
                        }
                        @if !flag.is_empty() {
                            span class="rn-flag" { (flag) }
                        }
                    }
                }
            }
            @if !link_ok.is_empty() {
                div class="rn-links" {
                    dt { (s.reader_note_links) }
                    dd {
                        ul {
                            @for l in &link_ok {
                                li { a href=(l) rel="nofollow noopener" target="_blank" { (l) } }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_index(lang: Lang, st: &AppState, notes: &[NoteCard]) -> Markup {
    let s = lang.s();
    html! {
        section class="narrow" {
            h1 { (s.reader_notes_title) }
            p class="lead" { (s.reader_notes_intro) }
            p { a class="btn btn-primary" href={(format!("/{}/reader-notes/submit", lang.code()))} { (s.reader_notes_cta) } }
            (ui::note_list(lang, notes, "reader-notes"))
        }
    }
}

pub async fn index(
    State(st): State<AppState>,
    Path(lang): Path<String>,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let notes = load_notes(&st.db, lang, "reader-notes", 50, 0).await?;
    let path = format!("/{}/reader-notes", lang.code());
    Ok(page_response(
        ui::page(
            lang,
            &path,
            s.reader_notes_title,
            s.reader_notes_intro,
            "reader-notes",
            &st.cfg.site_url,
            render_index(lang, &st, &notes),
        ),
        None,
        &st,
    ))
}

pub async fn detail(
    State(st): State<AppState>,
    Path((lang, slug)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    crate::public::detail_impl(st, lang, slug, headers, "reader-notes").await
}

fn form_page(
    lang: Lang,
    st: &AppState,
    v: Values,
    e: &Errors,
    csrf: &str,
    status: StatusCode,
    new: Option<String>,
) -> Response {
    let s = lang.s();
    let action = format!("/{}/reader-notes/submit", lang.code());
    let (qa, qb) = quiz_numbers();
    let weekday = if v.weekday.is_empty() {
        default_weekday(&v.occurred_at)
    } else {
        v.weekday.clone()
    };
    let weather_options = [
        ("", if lang == Lang::Zh { "不填" } else { "—" }),
        ("sunny", s.rn_weather_sunny),
        ("cloudy", s.rn_weather_cloudy),
        ("overcast", s.rn_weather_overcast),
        ("rain", s.rn_weather_rain),
        ("snow", s.rn_weather_snow),
    ];
    let color_options = [
        ("", s.rn_color_none),
        ("apricot", s.rn_color_apricot),
        ("lilac", s.rn_color_lilac),
        ("indigo", s.rn_color_indigo),
        ("gray", s.rn_color_gray),
    ];
    let flag_options = [
        ("", s.rn_flag_none),
        ("star", s.rn_flag_star),
        ("pennant", s.rn_flag_pennant),
        ("diamond", s.rn_flag_diamond),
        ("dot", s.rn_flag_dot),
    ];

    let content = html! {
        section class="rn-form" {
            h1 { (s.rn_form_title) }
            p class="lead" { (s.rn_form_intro) }
            div class="rn-draft" id="rn-draft" {
                span { (s.rn_draft_restored) }
                button class="btn btn-sm" type="button" id="rn-draft-clear" { (s.rn_draft_clear) }
            }
            @if e.any() {
                (ui::notice("warn", s.rn_err_generic))
                @if e.title { p class="err" { (s.rn_err_title) } }
                @if e.isbn { p class="err" { (s.rn_err_isbn) } }
                @if e.body { p class="err" { (s.rn_err_body_long) } }
                @if e.rate { p class="err" { (s.rn_err_rate) } }
                @if e.quiz { p class="err" { (s.rn_err_quiz) } }
                @if e.links {
                    p class="err" { (s.rn_err_link) }
                    @if e.bad_link.is_empty() {
                        p class="err" { (s.rn_err_links_count) }
                    }
                    @for b in &e.bad_link {
                        p class="err" { "· " (b) }
                    }
                }
            }
            form method="post" action=(action) id="rn-form" class="rn-form" novalidate {
                input type="hidden" name="csrf" value=(csrf);
                input type="hidden" name="quiz_a" value=(qa);
                input type="hidden" name="quiz_b" value=(qb);
                div class="hp" aria-hidden="true" {
                    label { "Website" }
                    input type="text" name="website" tabindex="-1" autocomplete="off";
                }

                div class="rn-grid three" {
                    div class={"field" (if e.long { " has-error" } else { "" })} {
                        label for="occurred_at" { (s.rn_date) " " span class="opt" { (s.rn_optional) } }
                        input id="occurred_at" name="occurred_at" type="datetime-local"
                              value=(if v.occurred_at.is_empty() { default_datetime() } else { v.occurred_at.clone() });
                    }
                    div class="field" {
                        label for="weekday" { (s.rn_weekday) " " span class="opt" { (s.rn_optional) } }
                        select id="weekday" name="weekday" {
                            @for (i, label) in s.rn_weekdays.iter().enumerate() {
                                option value=(i) selected[weekday == i.to_string()] { (label) }
                            }
                        }
                    }
                    div class="field" {
                        label for="place" { (s.rn_place) " " span class="opt" { (s.rn_optional) } }
                        input id="place" name="place" type="text" maxlength="80" value=(v.place)
                              placeholder=(s.rn_place_ph) autocomplete="off";
                    }
                }

                div class="rn-grid three" {
                    div class="field" {
                        label for="weather" { (s.rn_weather) " " span class="opt" { (s.rn_optional) } }
                        select id="weather" name="weather" {
                            @for (val, label) in weather_options {
                                option value=(val) selected[v.weather == val] { (label) }
                            }
                        }
                    }
                    div class="field" {
                        label for="temp_c" { (s.rn_temp) }
                        input id="temp_c" name="temp_c" type="text" maxlength="8" value=(v.temp_c)
                              placeholder="18℃" autocomplete="off";
                    }
                    div class="field" {
                        label for="book_title" { (s.rn_book) " " span class="opt" { (s.rn_optional) } }
                        input id="book_title" name="book_title" type="text" maxlength="160" value=(v.book_title);
                    }
                }

                div class={"field" (if e.isbn { " has-error" } else { "" })} {
                    label for="isbn" { (s.rn_isbn) }
                    input id="isbn" name="isbn" type="text" maxlength="20" value=(v.isbn)
                          inputmode="numeric" autocomplete="off";
                    p class="hint" { (s.rn_isbn_hint) }
                }

                div class={"field" (if e.title { " has-error" } else { "" })} {
                    label for="title" { (s.rn_title) " " span class="req" { (s.rn_required) } }
                    input id="title" name="title" type="text" maxlength="120" value=(v.title) required;
                }

                div class={"field" (if e.body { " has-error" } else { "" })} {
                    label for="body" { (s.rn_body) " " span class="opt" { (s.rn_optional) } }
                    textarea id="body" name="body" rows="10" maxlength="20000" { (v.body) }
                    p class="hint" { (s.rn_body_hint) }
                }

                div class="field" {
                    label for="question" { (s.rn_question) " " span class="opt" { (s.rn_optional) } }
                    input id="question" name="question" type="text" maxlength="280" value=(v.question);
                }

                div class="field" {
                    span class="label" { (s.rn_color) " " span class="opt" { (s.rn_optional) } }
                    div class="chip-set" {
                        @for (val, label) in color_options {
                            label class="chip" {
                                input type="radio" name="color" value=(val) checked[v.color == val];
                                span { (label) }
                            }
                        }
                    }
                }

                div class="field" {
                    span class="label" { (s.rn_flag) " " span class="opt" { (s.rn_optional) } }
                    div class="chip-set" {
                        @for (val, label) in flag_options {
                            label class="chip" {
                                input type="radio" name="flag" value=(val) checked[v.flag == val];
                                span { (label) }
                            }
                        }
                    }
                }

                div class={"field" (if e.links { " has-error" } else { "" })} {
                    label for="links" { (s.rn_links) }
                    textarea id="links" name="links" rows="3" { (v.links) }
                    p class="hint" { (s.rn_links_hint) }
                }

                div class={"field" (if e.quiz { " has-error" } else { "" })} {
                    label for="quiz" { (s.rn_quiz) }
                    div class="rn-quiz" {
                        span class="q" { (qa) " + " (qb) " = ?" }
                        input id="quiz" name="quiz" type="text" inputmode="numeric" maxlength="4"
                              autocomplete="off" required;
                    }
                    p class="hint" { (s.rn_quiz_hint) }
                }

                div class="field" {
                    button class="btn btn-primary" type="submit" { (s.rn_submit) }
                }
            }
        }
    };
    let mut resp = page_response(
        ui::page(
            lang,
            &format!("/{}/reader-notes/submit", lang.code()),
            s.rn_form_title,
            s.rn_form_intro,
            "reader-notes",
            &st.cfg.site_url,
            content,
        ),
        new,
        st,
    );
    *resp.status_mut() = status;
    resp
}

pub async fn submit_page(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let (csrf, new) = csrf_for(&headers);
    let dt = default_datetime();
    let v = Values {
        weekday: default_weekday(&dt),
        occurred_at: dt,
        ..Default::default()
    };
    Ok(form_page(
        lang,
        &st,
        v,
        &Errors::default(),
        &csrf,
        StatusCode::OK,
        new,
    ))
}

async fn lookup_region(st: &AppState, ip: &str) -> String {
    if !st.cfg.ip_geo || ip.is_empty() || ip == "unknown" || ip.starts_with("127.") || ip == "::1" {
        return String::new();
    }
    let ip = ip.to_string();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("curl")
            .args([
                "-s",
                "--max-time",
                "2",
                &format!("https://ipinfo.io/{}/json", ip),
            ])
            .output()
    })
    .await;
    let Ok(Ok(out)) = output else {
        return String::new();
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return String::new();
    };
    let pick = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let parts: Vec<String> = [pick("country"), pick("region"), pick("city")]
        .into_iter()
        .filter(|x| !x.is_empty() && x != "null")
        .collect();
    parts.join(" · ")
}

async fn next_receipt(db: &SqlitePool) -> Result<String, sqlx::Error> {
    let day = china_now().format("%Y%m%d").to_string();
    let like = format!("WH-{}-%", day);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM note_submissions WHERE receipt LIKE ?")
        .bind(&like)
        .fetch_one(db)
        .await?;
    Ok(format!("WH-{}-{:03}", day, count + 1))
}

pub async fn submit(
    State(st): State<AppState>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    axum::Form(f): axum::Form<SubmitForm>,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let (csrf, new) = csrf_for(&headers);
    let ip = util::client_ip(&headers);

    let (v, e) = validate(&f);

    // 蜜罐：不告知，直接给成功页；不落库。
    if !f.website.trim().is_empty() {
        return Ok(success_page(lang, &st, "WH-00000000-000", &new));
    }
    if !csrf_ok(&headers, &f.csrf) {
        return Ok(not_found_page(lang, &st.cfg.site_url));
    }

    let mut errors = e;
    if !quiz_ok(&f) {
        errors.quiz = true;
    }
    if errors.any() {
        return Ok(form_page(
            lang,
            &st,
            v,
            &errors,
            &csrf,
            StatusCode::UNPROCESSABLE_ENTITY,
            None,
        ));
    }

    let key = format!("rn:{}", util::ip_hash(&headers));
    if !st
        .limiter
        .allow(&key, RATE_MAX, Duration::from_secs(RATE_WINDOW_SECS))
    {
        return Ok(form_page(
            lang,
            &st,
            v,
            &Errors {
                rate: true,
                ..Default::default()
            },
            &csrf,
            StatusCode::TOO_MANY_REQUESTS,
            None,
        ));
    }

    let ip_hash = util::ip_hash(&headers);
    let region = lookup_region(&st, &ip).await;
    let now = china_now().format("%Y-%m-%d %H:%M:%S").to_string();
    let weekday = v.weekday.parse::<i64>().ok();
    let occurred_at = if v.occurred_at.is_empty() {
        now.clone()
    } else {
        v.occurred_at.clone()
    };
    let original = serde_json::json!({
        "occurred_at": occurred_at,
        "weekday": weekday,
        "place": v.place,
        "weather": v.weather,
        "temp_c": v.temp_c,
        "book_title": v.book_title,
        "isbn": v.isbn,
        "title": v.title,
        "body": v.body,
        "question": v.question,
        "color": v.color,
        "flag": v.flag,
        "links": v.links,
    })
    .to_string();

    let mut receipt = next_receipt(&st.db).await?;
    let mut inserted = false;
    for attempt in 0..5 {
        let res = sqlx::query(
            "INSERT INTO note_submissions
             (receipt, occurred_at, weekday, place, weather, temp_c, book_title, isbn,
              title, body, question, color, flag, links, lang, status, ip_hash, ip_region,
              created_at, original_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?)",
        )
        .bind(&receipt)
        .bind(&occurred_at)
        .bind(weekday)
        .bind(&v.place)
        .bind(&v.weather)
        .bind(&v.temp_c)
        .bind(&v.book_title)
        .bind(&v.isbn)
        .bind(&v.title)
        .bind(&v.body)
        .bind(&v.question)
        .bind(&v.color)
        .bind(&v.flag)
        .bind(&v.links)
        .bind(lang.code())
        .bind(&ip_hash)
        .bind(&region)
        .bind(&now)
        .bind(&original)
        .execute(&st.db)
        .await;
        match res {
            Ok(_) => {
                inserted = true;
                break;
            }
            Err(err) if attempt < 4 => {
                tracing::warn!("receipt collision {:?}: {}", receipt, err);
                let count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM note_submissions WHERE receipt LIKE ?",
                )
                .bind(format!("WH-{}-%", china_now().format("%Y%m%d")))
                .fetch_one(&st.db)
                .await?;
                receipt = format!(
                    "WH-{}-{:03}",
                    china_now().format("%Y%m%d"),
                    count + 1 + attempt as i64
                );
            }
            Err(err) => return Err(AppError(err.into())),
        }
    }
    if !inserted {
        return Err(AppError(anyhow::anyhow!("receipt generation failed")));
    }

    if st.cfg.submit_notify {
        let subject = format!("[wayhug] 新读者笔记待审 {} · {}", receipt, v.title);
        let body = format!(
            "回执：{}\n标题：{}\n书名：{}\n字数：{}\n提交时间：{}\n来源：{}\n\n请到 https://wayhug.com/admin/review 审阅。\n",
            receipt,
            v.title,
            if v.book_title.is_empty() { "—" } else { &v.book_title },
            v.title.chars().count() + v.body.chars().count() + v.question.chars().count(),
            now,
            if region.is_empty() { "未知" } else { &region }
        );
        st.mailer
            .send("submit", &st.cfg.contact_email, &subject, &body)
            .await;
    }

    Ok(success_page(lang, &st, &receipt, &new))
}

fn success_page(lang: Lang, st: &AppState, receipt: &str, new: &Option<String>) -> Response {
    let s = lang.s();
    let content = html! {
        section class="narrow" id="rn-success" data-clear-draft="1" {
            h1 { (s.rn_ok_title) }
            p { (s.rn_ok_body) "：" }
            p class="receipt" { (receipt) }
            p { (s.rn_ok_hint) }
            p { a class="btn" href={(format!("/{}/reader-notes", lang.code()))} { (s.reader_notes_title) } }
        }
    };
    let resp = page_response(
        ui::page(
            lang,
            &format!("/{}/reader-notes", lang.code()),
            s.rn_ok_title,
            s.rn_ok_body,
            "reader-notes",
            &st.cfg.site_url,
            content,
        ),
        new.clone(),
        st,
    );
    resp
}

pub async fn unsubscribe_page(
    State(st): State<AppState>,
    Path(lang): Path<String>,
) -> Result<Response, AppError> {
    let Some(lang) = Lang::from_seg(&lang) else {
        return Ok(not_found_page(Lang::Zh, &st.cfg.site_url));
    };
    let s = lang.s();
    let content = html! {
        section class="narrow" {
            h1 { (s.unsubscribe_title) }
            p class="lead" { (s.unsubscribe_body) }
            p { (s.unsubscribe_body2) }
            p { a class="btn" href={(format!("/{}/subscribe", lang.code()))} { (s.nav_subscribe) } }
        }
    };
    Ok(page_response(
        ui::page(
            lang,
            &format!("/{}/unsubscribe", lang.code()),
            s.unsubscribe_title,
            s.unsubscribe_body,
            "",
            &st.cfg.site_url,
            content,
        ),
        None,
        &st,
    ))
}

/// 后台实时预览：与前台完全相同的渲染路径。
pub async fn preview_render(body: &str, question: &str) -> String {
    let body_html = md::render(body);
    format!(
        "<div class=\"prose\">{}</div>{}",
        body_html,
        if question.trim().is_empty() {
            String::new()
        } else {
            format!(
                "<div class=\"rn-question\"><h2>一个思考或问题</h2><p>{}</p></div>",
                maud::html! { (question) }.into_string()
            )
        }
    )
}
