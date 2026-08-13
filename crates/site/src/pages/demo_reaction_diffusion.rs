use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Reaction-Diffusion" }
        p .prose {
            "Two chemicals diffuse across a grid at different rates; one converts the other on contact. "
            "That single rule, iterated, produces patterns that look like coral, cell division, or fingerprints. "
            "The whole simulation runs in Rust compiled to WebAssembly — click the canvas to disturb it."
        }

        canvas #rd-canvas .rd-canvas aria-label="Reaction-diffusion simulation" {}

        div .rd-controls {
            button #rd-toggle .theme-toggle type="button" { "Pause" }
            button #rd-reset .theme-toggle type="button" { "Reset" }
            @for (id, label) in [("coral", "Coral"), ("mitosis", "Mitosis"), ("solitons", "Solitons"), ("worms", "Worms")] {
                button .theme-toggle type="button" data-preset=(id) { (label) }
            }
            span #rd-status .mono { "loading" }
        }

        noscript {
            p .prose { "This demo needs JavaScript and WebAssembly. The rest of the site works without either." }
        }

        script type="module" src="/demos/loader.js" {}
    };
    shell::layout(site, Route::Demos, "Reaction-Diffusion", main)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_page_has_a_canvas_and_loads_the_module_lazily() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(r#"id="rd-canvas""#), "canvas missing");
        assert!(out.contains(r#"type="module""#), "module script missing");
        assert!(out.contains("/demos/loader.js"), "loader not referenced");
        assert!(out.contains(r#"data-preset="coral""#), "presets missing");
        assert!(out.contains("noscript"), "no fallback for JS-disabled visitors");
        assert!(
            out.contains(r#"aria-label="Reaction-diffusion simulation""#),
            "canvas missing an accessible label"
        );
    }

    #[test]
    fn demo_page_marks_demos_as_the_current_nav_item() {
        let out = render(&crate::content::fixture_site()).into_string();
        assert!(out.contains(r#"href="/demos/" aria-current="page""#));
    }
}
