use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Aho–Corasick" }
        p .prose {
            "The string-matching automaton behind the transaction-tagging rewrite on the projects page. "
            "Patterns build a trie; failure links let a single pass over the text find every match. "
            "Edit the patterns, then step or play the scan."
        }

        div .ac-controls {
            input #ac-patterns .demo-input type="text" value="he, she, his, hers" aria-label="Patterns, comma-separated";
            input #ac-text .demo-input type="text" value="ushers say she sells seashells" aria-label="Text to scan";
            button #ac-rebuild .theme-toggle type="button" { "Rebuild" }
            button #ac-play .theme-toggle type="button" { "Play" }
            button #ac-step .theme-toggle type="button" { "Step" }
            button #ac-reset .theme-toggle type="button" { "Reset" }
            span #ac-status .demo-status { "loading" }
        }

        canvas #ac-canvas .ac-canvas aria-label="Aho-Corasick automaton graph" {}

        div #ac-scan {}

        p .mono { "solid = trie edge · dashed = failure link · filled = pattern end" }
        p .prose { "Lowercase a–z only, up to 8 patterns of 10 letters; text up to 200 characters." }

        noscript {
            p .prose { "This demo needs JavaScript and WebAssembly. The rest of the site works without either." }
        }

        script type="module" src="/demos/aho-loader.js" {}
    };
    shell::sub_page(site, Route::Demos, crate::pages::demos::AHO_CORASICK, "Aho–Corasick", main)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_page_has_a_canvas_and_loads_the_module_lazily() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(r#"id="ac-canvas""#), "canvas missing");
        assert!(out.contains(r#"type="module""#), "module script missing");
        assert!(out.contains("/demos/aho-loader.js"), "loader not referenced");
        assert!(out.contains("noscript"), "no fallback for JS-disabled visitors");
        assert!(
            out.contains(r#"aria-label="Aho-Corasick automaton graph""#),
            "canvas missing an accessible label"
        );
    }

    #[test]
    fn demo_page_has_pattern_and_text_controls_with_defaults() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(r#"id="ac-patterns""#), "pattern input missing");
        assert!(out.contains(r#"value="he, she, his, hers""#), "default patterns missing");
        assert!(out.contains(r#"id="ac-text""#), "text input missing");
        assert!(
            out.contains(r#"value="ushers say she sells seashells""#),
            "default scan text missing"
        );
        assert!(out.contains(r#"id="ac-rebuild""#));
        assert!(out.contains(r#"id="ac-play""#));
        assert!(out.contains(r#"id="ac-step""#));
        assert!(out.contains(r#"id="ac-reset""#));
        assert!(out.contains(r#"id="ac-status""#));
        assert!(out.contains(r#"id="ac-scan""#), "scan text container missing");
    }

    #[test]
    fn demo_page_marks_demos_as_the_current_nav_item() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(r#"href="/demos/" aria-current="page""#));
    }
}
