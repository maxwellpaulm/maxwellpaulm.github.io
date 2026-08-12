use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// The persistent left rail from composition A. Collapses to a top bar
/// under 640px via the stylesheet. `current` is `None` on pages that aren't
/// any `Route` — the 404 page — so nothing in the nav is marked current.
pub fn rail(site: &Site, current: Option<Route>) -> Markup {
    html! {
        div .rail {
            div .mono { "PM" }
            nav aria-label="Primary" {
                @for route in Route::ALL {
                    @if Some(route) == current {
                        a href=(route.path()) aria-current="page" { (route.label()) }
                    } @else {
                        a href=(route.path()) { (route.label()) }
                    }
                }
            }
            div .rail-foot {
                div .mono { (site.location) }
                button type="button" .theme-toggle id="theme-toggle" aria-label="Toggle dark mode" aria-pressed="false" {
                    "Dark"
                }
                script {
                    (maud::PreEscaped(
                        r#"(function(){
var b=document.getElementById("theme-toggle");
var sync=function(){
var dark=document.documentElement.dataset.theme==="dark";
b.setAttribute("aria-pressed",dark?"true":"false");
b.textContent=dark?"Light":"Dark";
};
sync();
b.addEventListener("click",function(){
var dark=document.documentElement.dataset.theme==="dark";
if(dark){delete document.documentElement.dataset.theme}else{document.documentElement.dataset.theme="dark"}
try{localStorage.setItem("theme",dark?"light":"dark")}catch(e){}
sync();
});
})();"#
                    ))
                }
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
        let out = rail(&site(), Some(Route::About)).into_string();
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
    fn rail_marks_nothing_current_when_given_none() {
        let out = rail(&site(), None).into_string();
        for r in Route::ALL {
            assert!(out.contains(r.label()), "missing nav entry {}", r.label());
        }
        assert!(
            !out.contains("aria-current=\"page\""),
            "no nav entry should be marked current: {out}"
        );
    }

    #[test]
    fn rail_shows_location() {
        let out = rail(&site(), Some(Route::Index)).into_string();
        assert!(out.contains("Washington, DC"));
    }

    #[test]
    fn rail_renders_an_accessible_theme_toggle_button() {
        let out = rail(&site(), Some(Route::Index)).into_string();
        assert!(out.contains("<button"), "toggle must be a real button element");
        assert!(
            out.contains(r#"aria-label="Toggle dark mode""#),
            "missing accessible name: {out}"
        );
        assert!(out.contains("aria-pressed"), "missing aria-pressed state: {out}");
    }

    #[test]
    fn rail_button_defaults_to_the_server_rendered_light_state_label() {
        let out = rail(&site(), Some(Route::Index)).into_string();
        let btn_start = out.find("<button").expect("button present");
        let btn_end = out.find("</button>").expect("closing button tag present") + "</button>".len();
        let button = &out[btn_start..btn_end];
        assert!(
            button.contains(">Dark<"),
            "button should default to \"Dark\" (the site is server-rendered light): {button}"
        );
    }

    #[test]
    fn toggle_script_syncs_both_aria_pressed_and_the_visible_label() {
        let out = rail(&site(), Some(Route::Index)).into_string();
        // A double-quoted fragment, verbatim: if PreEscaped were removed, the
        // `"` characters would come out as `&quot;` and this would fail.
        assert!(
            out.contains(r#"document.getElementById("theme-toggle")"#),
            "missing toggle script: {out}"
        );
        assert!(
            out.contains(r#"b.textContent=dark?"Light":"Dark";"#),
            "sync function does not update the button's visible label: {out}"
        );
        assert!(
            !out.contains("&quot;"),
            "toggle script body was HTML-escaped, corrupting the JS: {out}"
        );
    }
}
