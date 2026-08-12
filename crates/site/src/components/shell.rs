use crate::components::rail::rail;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup, DOCTYPE};

/// Wraps page content in the full HTML document.
pub fn page(site: &Site, route: Route, title: &str, body: Markup) -> Markup {
    let canonical = format!("{}{}", site.url, route.path());
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
                link rel="canonical" href=(canonical);
                link rel="icon" type="image/svg+xml" href="/favicon.svg";
                link rel="apple-touch-icon" href="/apple-touch-icon.png";
                meta property="og:type" content="website";
                meta property="og:site_name" content=(site.name);
                meta property="og:title" content=(full_title);
                meta property="og:description" content=(site.description);
                meta property="og:url" content=(canonical);
                meta property="og:image" content=(og_image);
                meta name="twitter:card" content="summary_large_image";
                link rel="stylesheet" href="/style.css";
                script {
                    (maud::PreEscaped(
                        "try{var t=localStorage.getItem('theme');if(t==='dark')document.documentElement.dataset.theme='dark'}catch(e){}"
                    ))
                }
            }
            body {
                (body)
            }
        }
    }
}

/// Composition A: rail plus main column, wrapped in the document shell.
pub fn layout(site: &Site, current: Route, title: &str, main: Markup) -> Markup {
    page(
        site,
        current,
        title,
        html! {
            div .layout {
                (rail(site, current))
                main { (main) }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixture_site;

    #[test]
    fn page_emits_a_complete_document() {
        let site = fixture_site();
        let out = page(&site, Route::About, "About", html! { p { "hello" } }).into_string();
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
        let out = page(&site, Route::Index, "Index", html! { p { "x" } }).into_string();
        // site.url itself is https:// (canonical/OG); nothing else should be.
        for host in ["http://", "https://fonts.", "cdn.", "googleapis"] {
            assert!(!out.contains(host), "external reference {host} found in output");
        }
    }

    #[test]
    fn page_emits_description_canonical_and_icons() {
        let site = fixture_site();
        let out = page(&site, Route::About, "About", html! { p { "x" } }).into_string();
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
        let index = page(&site, Route::Index, "Index", html! { p { "x" } }).into_string();
        let about = page(&site, Route::About, "About", html! { p { "x" } }).into_string();

        let index_canonical = format!(r#"<link rel="canonical" href="{}/">"#, site.url);
        let about_canonical = format!(r#"<link rel="canonical" href="{}/about/">"#, site.url);

        assert!(index.contains(&index_canonical), "got: {index}");
        assert!(about.contains(&about_canonical), "got: {about}");
        assert_ne!(index_canonical, about_canonical);
    }

    #[test]
    fn page_emits_og_and_twitter_tags() {
        let site = fixture_site();
        let out = page(&site, Route::Projects, "Projects", html! { p { "x" } }).into_string();
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
        let out = page(&site, Route::Resume, "Resume", html! { p { "x" } }).into_string();
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
        let out = page(&site, Route::About, "About", html! { p { "x" } }).into_string();

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
        let out = page(&site, Route::Index, "Index", html! { p { "x" } }).into_string();
        assert!(
            out.contains("localStorage.getItem"),
            "missing inline theme-detection script: {out}"
        );
        let script_start = out.find("<script>").expect("script tag present");
        let script_end = out.find("</script>").expect("closing script tag present");
        let script_body = &out[script_start..script_end];
        assert!(
            !script_body.contains("&quot;"),
            "script body was HTML-escaped, corrupting the JS: {script_body}"
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
}
