use pulldown_cmark::{Event, Options, Parser, html};

/// 渲染 Markdown。原始 HTML 事件一律降级为文本，避免投稿内容注入脚本。
pub fn render(md: &str) -> String {
    let parser = Parser::new_ext(md, Options::empty()).map(|ev| match ev {
        Event::Html(s) | Event::InlineHtml(s) => Event::Text(s),
        other => other,
    });
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}
