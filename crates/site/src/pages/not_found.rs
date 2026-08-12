use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// `dist/404.html`. GitHub Pages serves this file for any request that
/// doesn't resolve to a real path — including inbound links built against
/// the pre-launch URL scheme (`/about.html`, now `/about/`). Deliberately
/// not a `Route`: see `shell::not_found` for why.
pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Page not found" }
        p .prose { "That link is out of date — the page moved or never existed." }
        p { a href=(Route::Index.path()) { "Back to the index" } }
    };
    shell::not_found(site, "Not Found", main)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixture_site;

    #[test]
    fn renders_a_heading_and_a_link_back_to_the_index() {
        let site = fixture_site();
        let out = render(&site).into_string();
        assert!(out.contains("<h1>Page not found</h1>"), "missing heading: {out}");
        assert!(
            out.contains(&format!(r#"href="{}""#, Route::Index.path())),
            "missing link back to the index: {out}"
        );
    }

    #[test]
    fn rail_marks_nothing_current() {
        let site = fixture_site();
        let out = render(&site).into_string();
        assert!(
            !out.contains(r#"aria-current="page""#),
            "404 rail must mark nothing current: {out}"
        );
    }
}
