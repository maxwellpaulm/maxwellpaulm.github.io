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
        assert_eq!(out.matches("<p class=\"prose\">").count(), site.about.len());
        assert!(out.contains("CFA charter"), "CFA belongs on About");
        assert!(out.contains("24 finishers"), "NSA line belongs on About");
        assert!(out.contains(r#"href="/about/" aria-current="page""#));
    }
}
