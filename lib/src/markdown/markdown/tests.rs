use super::*;

#[test]
fn default_markdown_extensions() {
    let markdown = Markdown::from("~~old~~ $x$ a^b^ c~d~".to_owned());
    let mut html = String::new();
    pulldown_cmark::html::push_html(
        &mut html,
        Parser::new_ext(&markdown.input, markdown.options),
    );
    assert_eq!(html, "<p><del>old</del> $x$ a^b^ c~d~</p>\n");
}
