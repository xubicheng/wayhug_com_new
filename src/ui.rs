use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::i18n::Lang;
use crate::models::{NoteCard, ProductRow};

pub fn page(
    lang: Lang,
    path: &str,
    title: &str,
    desc: &str,
    active: &str,
    site_url: &str,
    content: Markup,
) -> Markup {
    let s = lang.s();
    let home = format!("/{}/", lang.code());
    let switch = lang.switch_path(path);
    let full_title = if title.is_empty() {
        format!("wayhug.com · {}", s.site_tagline)
    } else {
        format!("{} · wayhug.com", title)
    };
    let canonical = format!("{}{}", site_url.trim_end_matches('/'), path);
    let skip = match lang {
        Lang::Zh => "跳到正文",
        Lang::En => "Skip to content",
    };

    html! {
        (DOCTYPE)
        html lang=(lang.html_lang()) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (full_title) }
                meta name="description" content=(desc);
                meta name="theme-color" content="#5d8aa8";
                link rel="canonical" href=(canonical);
                link rel="alternate" hreflang=(lang.other().html_lang()) href={
                    (format!("{}{}", site_url.trim_end_matches('/'), lang.switch_path(path)))
                };
                link rel="stylesheet" href="/static/site.css";
                link rel="icon" href="/static/favicon.svg" type="image/svg+xml";
            }
            body {
                a class="skip" href="#main" { (skip) }
                header class="site-header" {
                    div class="header-inner" {
                        a class="brand" href=(home) {
                            span class="brand-name" { "王元会" }
                            span class="brand-site" { "wayhug.com" }
                        }
                        nav class="nav" aria-label="导航 / Navigation" {
                            a class=(if active == "notes" { "active" } else { "" }) href={(format!("/{}/notes", lang.code()))} { (s.nav_notes) }
                            a class=(if active == "reader-notes" { "active" } else { "" }) href={(format!("/{}/reader-notes", lang.code()))} { (s.nav_reader_notes) }
                            a class=(if active == "readers" { "active" } else { "" }) href={(format!("/{}/readers", lang.code()))} { (s.nav_readers) }
                            a class=(if active == "products" { "active" } else { "" }) href={(format!("/{}/products", lang.code()))} { (s.nav_products) }
                            a class=(if active == "about" { "active" } else { "" }) href={(format!("/{}/about", lang.code()))} { (s.nav_about) }
                        }
                        a class="nav-cta" href={(format!("/{}/subscribe", lang.code()))} { (s.nav_subscribe) }
                        a class="lang-switch" href=(switch) hreflang=(lang.other().html_lang()) { (s.lang_switch) }
                    }
                }
                main id="main" class="wrap" { (content) }
                footer class="site-footer" {
                    div class="footer-grid" {
                        div class="footer-brand-col" {
                            a class="footer-mark" href=(home) {
                                picture {
                                    source type="image/avif" srcset="/static/brand-mark.avif 1x, /static/brand-mark@2x.avif 2x";
                                    source type="image/webp" srcset="/static/brand-mark.webp 1x, /static/brand-mark@2x.webp 2x";
                                    img src="/static/brand-mark.png" width="160" height="90" alt=(s.footer_brand_alt) loading="lazy" decoding="async";
                                }
                            }
                            p class="footer-brand" { "王元会 · wayhug.com" }
                            p class="footer-note" { (s.footer_note) }
                        }
                        div {
                            p class="footer-head" { (if lang == Lang::Zh { "站内" } else { "On this site" }) }
                            p { a href={(format!("/{}/notes", lang.code()))} { (s.nav_notes) } }
                            p { a href={(format!("/{}/reader-notes", lang.code()))} { (s.nav_reader_notes) } }
                            p { a href={(format!("/{}/readers", lang.code()))} { (s.nav_readers) } }
                            p { a href={(format!("/{}/products", lang.code()))} { (s.nav_products) } }
                            p { a href={(format!("/{}/about", lang.code()))} { (s.nav_about) } }
                        }
                        div class="footer-four" {
                            p class="footer-head" { (s.footer_four_title) }
                            p class="footer-four-note" { (s.footer_four_note) }
                            ul class="site-links" {
                                li {
                                    a href="https://jshnp.com/" target="_blank" rel="noopener" {
                                        span class="site-code" { "SJ" } " " (s.site_sj) " ↗"
                                    }
                                }
                                li {
                                    a href="https://37xxi.com/" target="_blank" rel="noopener" {
                                        span class="site-code" { "37" } " " (s.site_37) " ↗"
                                    }
                                }
                                li {
                                    a href="https://dynsh.com/" target="_blank" rel="noopener" {
                                        span class="site-code" { "NC" } " " (s.site_nc) " ↗"
                                    }
                                }
                            }
                        }
                        div class="footer-meta" {
                            p { a href={(format!("/{}/subscribe", lang.code()))} { (s.nav_subscribe) } }
                            p { a href={(format!("/{}/about#privacy", lang.code()))} { (s.footer_privacy) } }
                            p { a href={(format!("/{}/unsubscribe", lang.code()))} { (s.footer_unsubscribe) } }
                            p { (s.footer_ledger) }
                            p { a href="https://beian.miit.gov.cn/" rel="nofollow noopener" { (s.footer_icp) } }
                            p { a href="https://www.beian.gov.cn/" rel="nofollow noopener" { (s.footer_psb) } }
                        }
                    }
                    div class="footer-bottom" {
                        span { "© " (chrono::Utc::now().format("%Y")) " " (s.footer_rights) }
                        span class="footer-dot" { "·" }
                        a href=(home) { "wayhug.com" }
                    }
                }
                script src="/static/enhance.js" defer {}
            }
        }
    }
}

pub fn hero(lang: Lang) -> Markup {
    let s = lang.s();
    html! {
        section class="hero" {
            p class="kicker" { (s.home_kicker) }
            h1 { (s.site_tagline) }
            p class="lead" { (s.home_intro) }
            p class="hero-actions" {
                a class="btn" href={(format!("/{}/notes", lang.code()))} { (s.nav_notes) }
                a class="btn btn-quiet" href={(format!("/{}/subscribe", lang.code()))} { (s.nav_subscribe) }
            }
        }
    }
}

pub fn section_head(_lang: Lang, title: &str, more_href: Option<&str>, more_label: &str) -> Markup {
    html! {
        div class="section-head" {
            h2 { (title) }
            @if let Some(href) = more_href {
                a class="more" href=(href) { (more_label) " →" }
            }
        }
    }
}

pub fn note_list(lang: Lang, notes: &[NoteCard], section: &str) -> Markup {
    let s = lang.s();
    html! {
        ul class="note-list" {
            @for n in notes {
                li {
                    div class="note-meta" {
                        time datetime=(n.published_at.clone().unwrap_or_default()) {
                            (n.published_at.as_deref().map(|d| lang.date(d)).unwrap_or_default())
                        }
                        @if n.guest_author_id.is_some() {
                            span class="dot" { "·" }
                            span { (n.author_name.clone().unwrap_or_default()) }
                        }
                        @if n.has_lang == 0 && n.content_lang != lang.code() {
                            span class="dot" { "·" }
                            span class="badge" { "中文原文" }
                        }
                    }
                    h3 { a href={(format!("/{}/{}/{}", lang.code(), section, n.slug))} { (n.title) } }
                    p class="summary" { (n.summary) }
                }
            }
            @if notes.is_empty() {
                li class="empty" { (if section == "readers" { s.readers_empty } else { s.notes_empty }) }
            }
        }
    }
}

pub fn product_card(lang: Lang, p: &ProductRow) -> Markup {
    let s = lang.s();
    let price = format!("{}.{:02}", p.price_cents / 100, p.price_cents % 100);
    let stock = match p.stock.as_str() {
        "in_stock" => s.stock_in_stock,
        "preorder" => s.stock_preorder,
        _ => s.stock_sold_out,
    };
    html! {
        article class="product" {
            div class="product-head" {
                h3 { (p.name) }
                p class="price" { (if p.currency == "CNY" { "¥" } else { "" }) (price) span class="stock" { (stock) } }
            }
            p { (p.description) }
            p class="revenue" {
                strong { (s.revenue_use) "：" }
                (p.revenue_note)
            }
        }
    }
}

pub fn subscribe_form(lang: Lang, csrf: &str, action: &str) -> Markup {
    let s = lang.s();
    html! {
        form class="subscribe-form" method="post" action=(action) {
            input type="hidden" name="csrf" value=(csrf);
            div class="field" {
                label for="email" { (s.label_email) }
                input id="email" type="email" name="email" required maxlength="254" autocomplete="email" inputmode="email" placeholder="you@example.com";
            }
            div class="field" {
                label for="sub-lang" { "语言 / Language" }
                select id="sub-lang" name="lang" {
                    option value="zh" selected[lang == Lang::Zh] { "中文" }
                    option value="en" selected[lang == Lang::En] { "English" }
                }
            }
            div class="hp" aria-hidden="true" {
                label { "Website" }
                input type="text" name="website" tabindex="-1" autocomplete="off";
            }
            label class="check" {
                input type="checkbox" name="consent" value="1" required;
                span { (s.consent_label) }
            }
            button class="btn btn-primary" type="submit" { (s.button_subscribe) }
            p class="hint" { (s.consent_hint) }
            p class="hint" { (s.subscribe_privacy) }
        }
    }
}

pub fn notice(kind: &str, text: &str) -> Markup {
    html! {
        p class={"notice " (kind)} { (text) }
    }
}

pub fn prose(markdown_html: &str) -> Markup {
    PreEscaped(format!("<div class=\"prose\">{}</div>", markdown_html))
}
