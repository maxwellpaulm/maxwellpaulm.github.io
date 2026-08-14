use crate::components::{shell, work::work_list_detailed};
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Projects" }
        p .prose { (site.projects_intro) }
        div .section-head {
            span .mono { "Selected Work" }
            span .mono { (format!("{:02}", site.work.len())) }
        }
        (work_list_detailed(&site.work))
    };
    shell::layout(site, Route::Projects, "Projects", main)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixture_site;

    #[test]
    fn projects_lists_the_work_entries_without_placeholder_copy() {
        let site = fixture_site();
        let out = render(&site).into_string();
        assert!(out.contains("Transaction Tagging Engine"));
        assert!(out.contains(&site.projects_intro), "intro should come from site.toml");
        for banned in ["Project 1", "Description of your", "goes here", "Lorem"] {
            assert!(!out.contains(banned), "placeholder copy found: {banned}");
        }
    }

    #[test]
    fn projects_page_renders_the_detail_writeups() {
        let site = fixture_site();
        let out = render(&site).into_string();
        assert!(out.contains("forking the platform"), "Archie detail missing");
        assert!(out.contains("request-scoped tokens"), "Gateway detail missing");
        assert!(
            out.contains("without touching the pipeline"),
            "Duplicate-invoice detail missing"
        );
        assert!(out.contains("billion book titles"), "EU compliance detail missing");
        assert!(out.contains("hours to 90 seconds"), "Transaction tagging detail missing");
    }
}
