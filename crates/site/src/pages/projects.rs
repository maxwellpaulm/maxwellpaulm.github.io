use crate::components::{shell, work::work_list};
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

// TODO(task-7): remove once `build.rs` calls `render()` to place the
// projects page in the build output. No caller exists outside tests until
// then.
#[allow(dead_code)]
pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Projects" }
        p .prose { "Selected work, most recent first. Longer write-ups and interactive demos are on the way." }
        div .section-head {
            span .mono { "Selected Work" }
            span .mono { (format!("{:02}", site.work.len())) }
        }
        (work_list(&site.work))
    };
    shell::layout(site, Route::Projects, "Projects", main)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn projects_lists_the_work_entries_without_placeholder_copy() {
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = render(&site).into_string();
        assert!(out.contains("Transaction Tagging Engine"));
        for banned in ["Project 1", "Description of your", "goes here", "Lorem"] {
            assert!(!out.contains(banned), "placeholder copy found: {banned}");
        }
    }
}
