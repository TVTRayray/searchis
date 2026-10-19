//! md → `text/html` + `text/plain`。
//!
//! 原始 HTML 一律转义成文本；链接与图片地址只保留 http、https、mailto 与相对路径。
//! 生成的 HTML 只进入剪贴板，不在 Searchis 界面渲染。

use pulldown_cmark::{html, CowStr, Event, LinkType, Options, Parser, Tag};

use super::FormatSet;

fn allowed_url(url: &str) -> bool {
    // 浏览器会忽略 URL 中的空白与控制字符，所以先剔除再判断协议。
    let cleaned: String = url
        .chars()
        .filter(|ch| !ch.is_control() && !ch.is_whitespace())
        .collect();
    let lower = cleaned.to_ascii_lowercase();
    match lower.find(|ch| matches!(ch, ':' | '/' | '?' | '#')) {
        Some(index) if lower.as_bytes()[index] == b':' => {
            matches!(&lower[..index], "http" | "https" | "mailto")
        }
        _ => true,
    }
}

fn sanitize<'a>(event: Event<'a>) -> Event<'a> {
    match event {
        Event::Html(text) | Event::InlineHtml(text) => Event::Text(text),
        Event::Start(Tag::Link {
            link_type: _,
            dest_url,
            title,
            id,
        }) if !allowed_url(&dest_url) => Event::Start(Tag::Link {
            link_type: LinkType::Inline,
            dest_url: CowStr::Borrowed(""),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) if !allowed_url(&dest_url) => Event::Start(Tag::Image {
            link_type,
            dest_url: CowStr::Borrowed(""),
            title,
            id,
        }),
        other => other,
    }
}

/// Markdown 源码 → HTML 片段。
pub fn to_html(source: &str) -> String {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(source, options).map(sanitize);
    let mut out = String::with_capacity(source.len() * 2);
    html::push_html(&mut out, parser);
    out
}

pub fn convert(bytes: &[u8]) -> FormatSet {
    let lossy = String::from_utf8_lossy(bytes);
    let mut set = FormatSet::default();
    if matches!(lossy, std::borrow::Cow::Owned(_)) {
        set.notes
            .push("文件不是有效的 UTF-8，部分字符已替换。".into());
    }
    let html = format!(
        "<meta charset=\"utf-8\"><!--StartFragment-->{}<!--EndFragment-->",
        to_html(&lossy)
    );
    set.push("text/html", html.into_bytes());
    set.push("text/plain", lossy.as_bytes().to_vec());
    set.push("UTF8_STRING", lossy.as_bytes().to_vec());
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_headings_lists_code_blocks() {
        let html = to_html("# 标题\n\n- a\n- b\n\n```rust\nlet x = 1;\n```\n");
        assert!(html.contains("<h1>标题</h1>"));
        assert!(html.contains("<ul>") && html.contains("<li>a</li>"));
        assert!(html.contains("<pre><code class=\"language-rust\">let x = 1;"));
    }

    #[test]
    fn renders_gfm_tables_tasklists_and_strikethrough() {
        let html = to_html("|a|b|\n|-|-|\n|1|2|\n\n- [x] done\n- [ ] todo\n\n~~gone~~\n");
        assert!(html.contains("<table>") && html.contains("<th>a</th>") && html.contains("<td>2</td>"));
        assert!(html.contains("<input disabled=\"\" type=\"checkbox\" checked=\"\"/>"));
        assert!(html.contains("<del>gone</del>"));
    }

    #[test]
    fn raw_html_is_escaped_not_passed_through() {
        let html = to_html("<script>alert(1)</script>\n\nhello <b>x</b> <img src=x onerror=y>\n");
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<b>"));
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&lt;img src=x onerror=y&gt;"));
    }

    #[test]
    fn dangerous_link_schemes_are_removed_and_safe_ones_kept() {
        let html = to_html(
            "[a](javascript:alert(1)) [b](JaVa\tScript:x) [c](data:text/html,x) [d](https://e.com/x) [e](mailto:a@b.c) [f](docs/x.md) [g](#h) ![i](vbscript:x)",
        );
        assert!(!html.to_lowercase().contains("javascript"));
        assert!(!html.contains("data:"));
        assert!(!html.contains("vbscript"));
        assert!(html.contains("href=\"https://e.com/x\""));
        assert!(html.contains("href=\"mailto:a@b.c\""));
        assert!(html.contains("href=\"docs/x.md\""));
        assert!(html.contains("href=\"#h\""));
    }

    #[test]
    fn convert_provides_html_and_plain_source() {
        let set = convert("# 你好\n".as_bytes());
        let names: Vec<&str> = set.formats.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["text/html", "text/plain", "UTF8_STRING"]);
        assert_eq!(set.formats[1].1, "# 你好\n".as_bytes());
        assert!(set.notes.is_empty());
    }

    #[test]
    fn invalid_utf8_is_lossy_and_noted() {
        let set = convert(&[b'a', 0xFF, b'b']);
        assert_eq!(String::from_utf8(set.formats[1].1.clone()).unwrap(), "a\u{FFFD}b");
        assert_eq!(set.notes.len(), 1);
    }

    #[test]
    fn qa_perf_1mb_markdown_p95() {
        let block = "## 小节\n\n正文 **加粗** 与 `code`，[链接](https://example.com/a)。\n\n|a|b|\n|-|-|\n|1|2|\n\n- [x] 项\n\n";
        let source = block.repeat(1_048_576 / block.len() + 1);
        assert!(source.len() >= 1_048_576);
        let mut times: Vec<f64> = (0..100)
            .map(|_| {
                let start = std::time::Instant::now();
                convert(source.as_bytes());
                start.elapsed().as_secs_f64() * 1000.0
            })
            .collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!("NFR-PERF-06 md 1 MB: p95={:.1} ms max={:.1} ms", times[94], times[99]);
        assert!(times[94] < 1000.0);
    }
}
