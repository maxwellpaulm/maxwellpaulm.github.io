use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// Path of the reaction-diffusion demo page. Task 5 renders it.
pub const REACTION_DIFFUSION: &str = "/demos/reaction-diffusion/";

/// Path of the Aho-Corasick automaton visualizer demo page.
pub const AHO_CORASICK: &str = "/demos/aho-corasick/";

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Demos" }
        p .prose { "Small things built in Rust, compiled to WebAssembly, running in your browser." }
        div .section-head {
            span .mono { "Demos" }
            span .mono { "02" }
        }
        div .item {
            div {
                h3 { a href=(REACTION_DIFFUSION) { "Reaction-Diffusion" } }
                p { "A Gray-Scott simulation: two chemicals, one feeding on the other, painting patterns that look uncannily biological. Every pixel is computed in Rust, sixty times a second." }
            }
            div .mono .year { "2026" }
        }
        div .item {
            div {
                h3 { a href=(AHO_CORASICK) { "Aho–Corasick" } }
                p { "Build the automaton, watch failure links form, and stream text through it." }
            }
            div .mono .year { "2026" }
        }
    };
    shell::layout(site, Route::Demos, "Demos", main)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demos_index_links_to_the_reaction_diffusion_demo() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(REACTION_DIFFUSION), "demo link missing");
        assert!(out.contains("Reaction-Diffusion"));
        assert!(out.contains(r#"href="/demos/" aria-current="page""#));
    }

    #[test]
    fn demos_index_links_to_the_aho_corasick_demo() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(AHO_CORASICK), "demo link missing");
        assert!(out.contains("Aho–Corasick"));
        assert!(
            out.contains("Build the automaton, watch failure links form, and stream text through it."),
            "summary missing: {out}"
        );
    }

    #[test]
    fn demos_index_count_reflects_both_demos() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(r#"<span class="mono">02</span>"#), "count did not become 02: {out}");
    }
}
