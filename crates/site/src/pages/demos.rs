use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// Path of the reaction-diffusion demo page. Task 5 renders it.
pub const REACTION_DIFFUSION: &str = "/demos/reaction-diffusion/";

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Demos" }
        p .prose { "Small things built in Rust, compiled to WebAssembly, running in your browser." }
        div .section-head {
            span .mono { "Demos" }
            span .mono { "01" }
        }
        div .item {
            div {
                h3 { a href=(REACTION_DIFFUSION) { "Reaction-Diffusion" } }
                p { "A Gray-Scott simulation: two chemicals, one feeding on the other, painting patterns that look uncannily biological. Every pixel is computed in Rust, sixty times a second." }
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
}
