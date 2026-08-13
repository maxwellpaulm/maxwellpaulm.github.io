use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "About" }
        @for paragraph in &site.about {
            p .prose { (paragraph) }
        }
        p .prose {
            "Elsewhere: "
            a href=(site.github) rel="me" { "GitHub" }
            " · "
            a href=(site.linkedin) rel="me" { "LinkedIn" }
        }
    };
    shell::layout(site, Route::About, "About", main)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn about_renders_every_paragraph_and_mentions_the_cfa() {
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = render(&site).into_string();
        // One <p class="prose"> per about paragraph, plus the links line.
        assert_eq!(out.matches("<p class=\"prose\">").count(), site.about.len() + 1);
        assert!(out.contains("CFA charter"), "CFA belongs on About");
        assert!(out.contains("24 finishers"), "NSA line belongs on About");
        assert!(out.contains(r#"href="/about/" aria-current="page""#));
    }

    #[test]
    fn about_links_to_github_and_linkedin() {
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = render(&site).into_string();
        assert!(
            out.contains(r#"href="https://github.com/maxwellpaulm""#),
            "missing github href: {out}"
        );
        assert!(
            out.contains(r#"href="https://www.linkedin.com/in/maxwellpaulm""#),
            "missing linkedin href: {out}"
        );
        assert!(out.contains(">GitHub<"), "expected visible link text \"GitHub\": {out}");
        assert!(out.contains(">LinkedIn<"), "expected visible link text \"LinkedIn\": {out}");
        assert!(out.contains(r#"rel="me""#), "profile links should carry rel=\"me\": {out}");
        assert_eq!(
            out.matches(r#"rel="me""#).count(),
            2,
            "both github and linkedin links should carry rel=\"me\": {out}"
        );
    }
}
