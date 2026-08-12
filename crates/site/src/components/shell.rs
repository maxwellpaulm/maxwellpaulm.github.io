use crate::components::rail::rail;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup, DOCTYPE};

/// Wraps page content in the full HTML document. `route` is `None` for pages
/// that don't have one stable URL — the 404 page, served for every unmatched
/// path — in which case no `<link rel="canonical">` or `og:url` is emitted
/// (there is no correct URL to claim), and `<meta name="robots"
/// content="noindex">` is added instead, so the page is kept out of search
/// results outright rather than left to an absent-canonical inference.
pub fn page(site: &Site, route: Option<Route>, title: &str, body: Markup) -> Markup {
    let canonical = route.map(|r| format!("{}{}", site.url, r.path()));
    // Built once and reused for <title> and og:title so the two can never disagree.
    let full_title = format!("{title} · {}", site.name);
    let og_image = format!("{}/og-image.png", site.url);
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (full_title) }
                meta name="description" content=(site.description);
                @if let Some(canonical) = &canonical {
                    link rel="canonical" href=(canonical);
                } @else {
                    meta name="robots" content="noindex";
                }
                link rel="icon" type="image/svg+xml" href="/favicon.svg";
                link rel="apple-touch-icon" href="/apple-touch-icon.png";
                meta property="og:type" content="website";
                meta property="og:site_name" content=(site.name);
                meta property="og:title" content=(full_title);
                meta property="og:description" content=(site.description);
                @if let Some(canonical) = &canonical {
                    meta property="og:url" content=(canonical);
                }
                meta property="og:image" content=(og_image);
                meta name="twitter:card" content="summary_large_image";
                link rel="stylesheet" href="/style.css";
                script {
                    (maud::PreEscaped(
                        r#"try{var t=localStorage.getItem("theme");if(t==="dark")document.documentElement.dataset.theme="dark"}catch(e){}"#
                    ))
                }
            }
            body {
                (body)
            }
        }
    }
}

/// Composition A: rail plus main column. `nav_current` drives which nav entry
/// (if any) the rail marks with `aria-current`.
fn composition(site: &Site, nav_current: Option<Route>, main: Markup) -> Markup {
    html! {
        div .layout {
            (rail(site, nav_current))
            main { (main) }
        }
    }
}

/// Composition A: rail plus main column, wrapped in the document shell.
pub fn layout(site: &Site, current: Route, title: &str, main: Markup) -> Markup {
    page(site, Some(current), title, composition(site, Some(current), main))
}

/// The document shell for `dist/404.html`. GitHub Pages serves this file
/// verbatim for any request that doesn't match a real path, so it isn't tied
/// to a `Route`: `Route::ALL` drives the nav, the sitemap, and the build
/// loop, and a 404 entry belongs in none of those. Passing `None` through to
/// `page` also means no canonical/`og:url` (a 404 has no single stable URL
/// to claim) and a `noindex` directive; the rail marks nothing current.
pub fn not_found(site: &Site, title: &str, main: Markup) -> Markup {
    page(site, None, title, composition(site, None, main))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixture_site;

    #[test]
    fn page_emits_a_complete_document() {
        let site = fixture_site();
        let out = page(&site, Some(Route::About), "About", html! { p { "hello" } }).into_string();
        assert!(out.starts_with("<!DOCTYPE html>"), "missing doctype: {out}");
        assert!(out.contains(r#"<html lang="en">"#), "missing lang attribute");
        assert!(out.contains(&format!("<title>About · {}</title>", site.name)));
        assert!(out.contains(r#"<meta name="viewport""#), "missing viewport meta");
        assert!(out.contains(r#"<link rel="stylesheet" href="/style.css">"#));
        assert!(out.contains("<p>hello</p>"), "body content not rendered");
    }

    #[test]
    fn page_makes_no_external_requests() {
        let site = fixture_site();
        let out = page(&site, Some(Route::Index), "Index", html! { p { "x" } }).into_string();
        // site.url itself is https:// (canonical/OG); nothing else should be.
        for host in ["http://", "https://fonts.", "cdn.", "googleapis"] {
            assert!(!out.contains(host), "external reference {host} found in output");
        }
    }

    #[test]
    fn page_emits_description_canonical_and_icons() {
        let site = fixture_site();
        let out = page(&site, Some(Route::About), "About", html! { p { "x" } }).into_string();
        assert!(
            out.contains(&format!(r#"<meta name="description" content="{}">"#, site.description)),
            "missing description meta: {out}"
        );
        assert!(
            out.contains(&format!(r#"<link rel="canonical" href="{}/about/">"#, site.url)),
            "missing canonical link: {out}"
        );
        assert!(out.contains(r#"<link rel="icon" type="image/svg+xml" href="/favicon.svg">"#));
        assert!(out.contains(r#"<link rel="apple-touch-icon" href="/apple-touch-icon.png">"#));
    }

    #[test]
    fn canonical_url_differs_between_routes() {
        let site = fixture_site();
        let index = page(&site, Some(Route::Index), "Index", html! { p { "x" } }).into_string();
        let about = page(&site, Some(Route::About), "About", html! { p { "x" } }).into_string();

        let index_canonical = format!(r#"<link rel="canonical" href="{}/">"#, site.url);
        let about_canonical = format!(r#"<link rel="canonical" href="{}/about/">"#, site.url);

        assert!(index.contains(&index_canonical), "got: {index}");
        assert!(about.contains(&about_canonical), "got: {about}");
        assert_ne!(index_canonical, about_canonical);
    }

    #[test]
    fn page_emits_og_and_twitter_tags() {
        let site = fixture_site();
        let out = page(&site, Some(Route::Projects), "Projects", html! { p { "x" } }).into_string();
        assert!(out.contains(r#"<meta property="og:type" content="website">"#));
        assert!(out.contains(&format!(r#"<meta property="og:site_name" content="{}">"#, site.name)));
        assert!(out.contains(&format!(
            r#"<meta property="og:title" content="Projects · {}">"#,
            site.name
        )));
        assert!(out.contains(&format!(
            r#"<meta property="og:description" content="{}">"#,
            site.description
        )));
        assert!(out.contains(&format!(
            r#"<meta property="og:url" content="{}/projects/">"#,
            site.url
        )));
        assert!(out.contains(&format!(
            r#"<meta property="og:image" content="{}/og-image.png">"#,
            site.url
        )));
        assert!(out.contains(r#"<meta name="twitter:card" content="summary_large_image">"#));
    }

    #[test]
    fn og_title_agrees_with_the_document_title() {
        let site = fixture_site();
        let out = page(&site, Some(Route::Resume), "Resume", html! { p { "x" } }).into_string();
        assert!(out.contains(&format!("<title>Resume · {}</title>", site.name)));
        assert!(out.contains(&format!(
            r#"<meta property="og:title" content="Resume · {}">"#,
            site.name
        )));
    }

    #[test]
    fn title_and_og_title_come_from_site_name_not_a_hardcoded_literal() {
        let mut site = fixture_site();
        site.name = "Someone Else Entirely".to_string();
        let out = page(&site, Some(Route::About), "About", html! { p { "x" } }).into_string();

        assert!(
            out.contains("<title>About · Someone Else Entirely</title>"),
            "title did not follow site.name: {out}"
        );
        assert!(
            out.contains(r#"<meta property="og:title" content="About · Someone Else Entirely">"#),
            "og:title did not follow site.name: {out}"
        );
        // The old hardcoded implementation would have produced this exact
        // title regardless of site.name; its absence proves the name is no
        // longer baked into the format string.
        assert!(
            !out.contains("<title>About · Paul Maxwell</title>"),
            "title still built from a hardcoded literal, not site.name: {out}"
        );
    }

    #[test]
    fn head_applies_the_stored_theme_before_first_paint() {
        let site = fixture_site();
        let out = page(&site, Some(Route::Index), "Index", html! { p { "x" } }).into_string();
        // A double-quoted fragment, verbatim: if PreEscaped were removed, the
        // `"` characters would come out as `&quot;` and this would fail.
        assert!(
            out.contains(r#"localStorage.getItem("theme")"#),
            "missing inline theme-detection script: {out}"
        );
        assert!(
            !out.contains("&quot;"),
            "script body was HTML-escaped, corrupting the JS: {out}"
        );
    }

    #[test]
    fn layout_wraps_main_content_beside_the_rail() {
        use crate::route::Route;

        let site = fixture_site();
        let out = layout(&site, Route::Index, "Index", html! { p { "body" } }).into_string();
        assert!(out.contains(r#"class="layout""#));
        assert!(out.contains(r#"class="rail""#));
        assert!(out.contains("<main>"));
        assert!(out.contains("<p>body</p>"));
    }

    #[test]
    fn not_found_renders_the_full_document_with_nothing_marked_current() {
        let site = fixture_site();
        let out = not_found(&site, "Not Found", html! { p { "gone" } }).into_string();
        assert!(out.starts_with("<!DOCTYPE html>"), "missing doctype: {out}");
        assert!(out.contains(r#"class="layout""#));
        assert!(out.contains(r#"class="rail""#));
        assert!(out.contains("<p>gone</p>"));
        assert!(
            !out.contains(r#"aria-current="page""#),
            "404 rail must mark nothing current: {out}"
        );
    }

    #[test]
    fn page_without_a_route_omits_canonical_and_og_url_but_keeps_other_og_tags() {
        let site = fixture_site();
        let out = page(&site, None, "Not Found", html! { p { "x" } }).into_string();
        assert!(!out.contains(r#"rel="canonical""#), "canonical must be absent: {out}");
        assert!(!out.contains("og:url"), "og:url must be absent: {out}");
        assert!(
            out.contains(r#"<meta name="robots" content="noindex">"#),
            "missing noindex directive: {out}"
        );
        // The rest of the OG tags are still accurate for a page with no URL.
        assert!(out.contains(r#"<meta property="og:type" content="website">"#));
        assert!(out.contains(&format!(r#"<meta property="og:site_name" content="{}">"#, site.name)));
        assert!(out.contains(&format!(
            r#"<meta property="og:title" content="Not Found · {}">"#,
            site.name
        )));
        assert!(out.contains(&format!(
            r#"<meta property="og:description" content="{}">"#,
            site.description
        )));
        assert!(out.contains(&format!(r#"<meta property="og:image" content="{}/og-image.png">"#, site.url)));
    }

    #[test]
    fn not_found_page_has_no_canonical_or_og_url_and_is_noindexed() {
        let site = fixture_site();
        let out = not_found(&site, "Not Found", html! { p { "gone" } }).into_string();
        assert!(!out.contains(r#"rel="canonical""#), "canonical must be absent: {out}");
        assert!(!out.contains("og:url"), "og:url must be absent: {out}");
        assert!(
            out.contains(r#"<meta name="robots" content="noindex">"#),
            "missing noindex directive: {out}"
        );
    }
}
