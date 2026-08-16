use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// Path of the reaction-diffusion demo page. Task 5 renders it.
pub const REACTION_DIFFUSION: &str = "/demos/reaction-diffusion/";

/// Path of the Aho-Corasick automaton visualizer demo page.
pub const AHO_CORASICK: &str = "/demos/aho-corasick/";

struct DemoEntry {
    title: &'static str,
    href: &'static str,
    year: &'static str,
    summary: &'static str,
}

/// Every demo listed on this page. The section header's count is derived
/// from this slice's length (see `pages::index`/`pages::projects`, which
/// derive their own section counts from `site.work.len()` the same way)
/// rather than hand-typed, so adding or removing a demo can't silently
/// leave a stale count behind.
const DEMOS: [DemoEntry; 2] = [
    DemoEntry {
        title: "Reaction-Diffusion",
        href: REACTION_DIFFUSION,
        year: "2026",
        summary: "A Gray-Scott simulation: two chemicals, one feeding on the other, painting patterns that look uncannily biological. Every pixel is computed in Rust, sixty times a second.",
    },
    DemoEntry {
        title: "Aho–Corasick",
        href: AHO_CORASICK,
        year: "2026",
        summary: "Build the automaton, watch failure links form, and stream text through it.",
    },
];

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Demos" }
        p .prose { "Small things built in Rust, compiled to WebAssembly, running in your browser." }
        div .section-head {
            span .mono { "Demos" }
            span .mono { (format!("{:02}", DEMOS.len())) }
        }
        @for demo in DEMOS {
            div .item {
                div {
                    h3 { a href=(demo.href) { (demo.title) } }
                    p { (demo.summary) }
                }
                div .mono .year { (demo.year) }
            }
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
            out.contains(
                "Build the automaton, watch failure links form, and stream text through it."
            ),
            "summary missing: {out}"
        );
    }

    #[test]
    fn demos_index_count_reflects_the_number_of_rendered_items() {
        let out = render(&crate::content::fixture_site()).into_string();
        let expected = format!(r#"<span class="mono">{:02}</span>"#, DEMOS.len());
        assert!(out.contains(&expected), "count did not match DEMOS.len(): {out}");
        // Guards the derivation itself, not just the string it produces:
        // this would still pass a hardcoded "02" that drifted from a third
        // demo being added without a count bump.
        assert_eq!(DEMOS.len(), out.matches(r#"<div class="item">"#).count());
    }
}
