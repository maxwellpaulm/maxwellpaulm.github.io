use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// Published location of the PDF fetched from the private release.
pub const RESUME_PDF: &str = "/assets/paul_maxwell_resume.pdf";

pub fn render(site: &Site) -> Markup {
    let main = html! {
        h1 { "Resume" }
        p .prose {
            a href=(RESUME_PDF) download { "Download PDF" }
        }
        object data=(RESUME_PDF) type="application/pdf"
            style="width:100%;height:80vh;border:1px solid var(--rule);margin-top:1.5rem" {
            p .prose {
                "Your browser cannot display the embedded PDF. "
                a href=(RESUME_PDF) download { "Download it instead." }
            }
        }
    };
    shell::layout(site, Route::Resume, "Resume", main)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn resume_offers_a_download_and_embeds_the_pdf() {
        let site = Site::load(Path::new("../../content/site.toml")).unwrap();
        let out = render(&site).into_string();
        assert!(out.contains(&format!(r#"href="{RESUME_PDF}""#)), "download link missing");
        assert!(out.contains("download"), "download attribute missing");
        assert!(out.contains(RESUME_PDF), "embed source missing");
    }
}
