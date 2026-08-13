use crate::components::{shell, work::work_list};
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

pub fn render(site: &Site) -> Markup {
    let main = html! {
        // Rendered from `site.name` rather than hardcoded, so the field is
        // actually read — an unread struct field fails `clippy -D warnings`.
        h1 {
            @for part in site.name.split(' ') {
                span .name-line { (part) }
            }
        }
        p .lede { (site.lede) }
        p .prose { (site.bio) }
        p .mono style="margin-top:2rem" { (site.role) }

        div .section-head {
            span .mono { "Selected Work" }
            span .mono { (format!("{:02}", site.work.len())) }
        }
        (work_list(&site.work))
    };
    shell::layout(site, Route::Index, "Index", main)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn index_shows_name_lede_and_selected_work() {
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = render(&site).into_string();
        assert!(out.contains("Paul"), "name missing");
        assert!(out.contains("backend systems"), "lede missing");
        assert!(out.contains("Selected Work"));
        assert!(out.contains("Archie BYOC Platform"));
        assert!(out.contains(r#"aria-current="page""#));
    }
}
