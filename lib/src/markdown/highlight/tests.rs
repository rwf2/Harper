use pulldown_cmark::Parser;

use super::*;

fn render(markdown: &str) -> String {
    let mut html = String::new();
    let mut highlighter = SyntaxHighlight;
    let events = highlighter.remap(Parser::new(markdown));
    pulldown_cmark::html::push_html(&mut html, events);
    html
}

#[test]
fn highlights_known_language() {
    let html = render("```rust,ignore\nfn main() {}\n```\n");
    assert!(html.starts_with("<div class=\"code\" style=\"display: flex;\">"));
    assert!(html.contains("<pre class=\"line-nums\">1</pre>"));
    assert!(html.contains("<span class=\"keyword\">fn</span>"));
    assert!(html.contains("<span class=\"function\">main</span>"));
    assert!(html.contains("<span class=\"punctuation bracket\">(</span>"));
}

#[test]
fn escapes_unknown_language() {
    let html = render("```nope\n<b>&</b>\nx\n```\n");
    assert!(html.contains("<pre class=\"line-nums\">1\n2</pre>"));
    assert!(html.contains("<pre class=\"code\">&lt;b&gt;&amp;&lt;/b&gt;\nx\n</pre>"));
    assert!(!html.contains("<span"));
}

#[test]
fn escapes_highlighted_source() {
    let html = render("```rust\nlet s = \"<a>\";\n```\n");
    assert!(html.contains("&lt;a&gt;"));
    assert!(!html.contains("<a>"));
}
