use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// The persistent left rail from composition A. Collapses to a top bar
/// under 640px via the stylesheet.
pub fn rail(site: &Site, current: Route) -> Markup {
    html! {
        div .rail {
            div .mono { "PM" }
            nav aria-label="Primary" {
                @for route in Route::ALL {
                    @if route == current {
                        a href=(route.path()) aria-current="page" { (route.label()) }
                    } @else {
                        a href=(route.path()) { (route.label()) }
                    }
                }
            }
            div .rail-foot {
                div .mono { (site.location) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn site() -> Site {
        Site::load(Path::new("../../content/site.toml")).unwrap()
    }

    #[test]
    fn rail_lists_every_route_and_marks_the_current_one() {
        let out = rail(&site(), Route::About).into_string();
        for r in Route::ALL {
            assert!(out.contains(r.label()), "missing nav entry {}", r.label());
        }
        assert!(
            out.contains(r#"href="/about/" aria-current="page""#),
            "current page not marked: {out}"
        );
        assert!(!out.contains("Demos"), "Demos must not be linked in bucket 1");
    }

    #[test]
    fn rail_shows_location() {
        let out = rail(&site(), Route::Index).into_string();
        assert!(out.contains("Washington, DC"));
    }
}
