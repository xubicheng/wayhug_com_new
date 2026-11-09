use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};

/// 只允许 http/https 的绝对链接；其余（javascript:、data:、相对协议等）降级为纯文本。
fn safe_url(dest: &str) -> bool {
    let d = dest.trim().to_ascii_lowercase();
    d.starts_with("http://") || d.starts_with("https://")
}

/// 渲染 Markdown。原始 HTML 事件降级为文本；图片与不安全链接降级为文字，避免注入。
pub fn render(md: &str) -> String {
    let parser = Parser::new_ext(md, Options::empty());
    let mut filtered: Vec<Event> = Vec::new();
    let mut links: Vec<bool> = Vec::new();
    let mut image_depth = 0usize;

    for ev in parser {
        match ev {
            Event::Html(s) | Event::InlineHtml(s) => filtered.push(Event::Text(s)),
            Event::Start(tag) => match &tag {
                Tag::Link { dest_url, .. } => {
                    let ok = safe_url(dest_url);
                    links.push(ok);
                    if ok {
                        filtered.push(Event::Start(tag));
                    } else {
                        filtered.push(Event::Text("".into()));
                    }
                }
                Tag::Image { .. } => {
                    image_depth += 1;
                    if image_depth == 1 {
                        filtered.push(Event::Text("[图片]".into()));
                    }
                }
                _ => filtered.push(Event::Start(tag)),
            },
            Event::End(tag) => match tag {
                TagEnd::Link => {
                    if links.pop().unwrap_or(true) {
                        filtered.push(Event::End(tag));
                    } else {
                        filtered.push(Event::Text("".into()));
                    }
                }
                TagEnd::Image => {
                    image_depth = image_depth.saturating_sub(1);
                }
                _ => filtered.push(Event::End(tag)),
            },
            other => filtered.push(other),
        }
    }

    let mut out = String::new();
    html::push_html(&mut out, filtered.into_iter());
    out
}

/// 从 Markdown 提取纯文本摘要，保留中文标点。
pub fn plain(md: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let parser = Parser::new_ext(md, Options::empty());
    for ev in parser {
        match ev {
            Event::Text(s) | Event::Code(s) => out.push_str(&s),
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            Event::End(TagEnd::Paragraph) => out.push(' '),
            _ => {}
        }
    }
    let trimmed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.chars().count() <= max_chars {
        return trimmed;
    }
    let cut: String = trimmed.chars().take(max_chars).collect();
    format!("{}…", cut.trim_end())
}
