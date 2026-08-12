use maud::{html, Markup, DOCTYPE};

/// Wraps page content in the full HTML document.
// TODO(task-5): remove once `layout()` calls `page()`. Until then clippy
// flags it dead because this crate has no lib target and no caller.
#[allow(dead_code)]
pub fn page(title: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · Paul Maxwell" }
                link rel="stylesheet" href="/style.css";
            }
            body {
                (body)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_emits_a_complete_document() {
        let out = page("About", html! { p { "hello" } }).into_string();
        assert!(out.starts_with("<!DOCTYPE html>"), "missing doctype: {out}");
        assert!(out.contains(r#"<html lang="en">"#), "missing lang attribute");
        assert!(out.contains("<title>About · Paul Maxwell</title>"));
        assert!(out.contains(r#"<meta name="viewport""#), "missing viewport meta");
        assert!(out.contains(r#"<link rel="stylesheet" href="/style.css">"#));
        assert!(out.contains("<p>hello</p>"), "body content not rendered");
    }

    #[test]
    fn page_makes_no_external_requests() {
        let out = page("Index", html! { p { "x" } }).into_string();
        for host in ["http://", "https://fonts.", "cdn.", "googleapis"] {
            assert!(!out.contains(host), "external reference {host} found in output");
        }
    }
}
