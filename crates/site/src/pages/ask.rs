use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Ask" }
        p .prose {
            "Ask about my work. Answers come straight from this site's content, "
            "ranked in your browser by a Rust BM25 searcher compiled to WebAssembly — "
            "no server, no model, no tracking."
        }

        div #ask-log .ask-log {}

        form #ask-form .ask-form {
            span .ask-prompt aria-hidden="true" { ">" }
            input #ask-input .demo-input type="text" autocomplete="off"
                aria-label="Ask a question about Paul's work";
            button .theme-toggle type="submit" { "Ask" }
        }

        noscript {
            p .prose {
                "The ask terminal needs JavaScript and WebAssembly. Everything it "
                "knows is already on the about and projects pages."
            }
        }

        script type="module" src="/ask/terminal.js" {}
    };
    shell::layout(site, Route::Ask, "Ask", main)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixture_site;

    #[test]
    fn ask_page_wires_the_terminal() {
        let out = render(&fixture_site()).into_string();
        assert!(out.contains(r#"id="ask-log""#), "missing scrollback container: {out}");
        assert!(out.contains(r#"id="ask-input""#), "missing query input: {out}");
        assert!(
            out.contains(r#"<script type="module" src="/ask/terminal.js">"#),
            "missing module script (must be terminal.js, never loader.js): {out}"
        );
        assert!(out.contains("noscript"), "no fallback for JS-disabled visitors: {out}");
        assert!(!out.contains("loader.js"), "route pages must not reference any loader.js");
    }
}
