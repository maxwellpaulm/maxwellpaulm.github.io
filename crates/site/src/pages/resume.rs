use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// Published location of the PDF fetched from the private release.
pub const RESUME_PDF: &str = "/assets/paul_maxwell_resume.pdf";

/// Renders the resume as inline vector pages with a hidden text layer.
///
/// `pages` holds rooted URLs in page order; `text` is the plain-text
/// extraction. With no pages — a local build that has not run
/// `scripts/render-resume.sh` — the page degrades to the download link.
pub fn render(site: &Site, pages: &[String], text: &str) -> Markup {
    let main = html! {
        h1 { "Resume" }
        p .prose {
            a href=(RESUME_PDF) download { "Download PDF" }
        }

        @for (i, page) in pages.iter().enumerate() {
            img .resume-page src=(page) alt=(format!("Resume page {} of {}", i + 1, pages.len()));
        }

        @if !text.is_empty() {
            div .visually-hidden { (text) }
        }
    };
    shell::layout(site, Route::Resume, "Resume", main)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages() -> Vec<String> {
        vec!["/resume/page-01.svg".to_string(), "/resume/page-02.svg".to_string()]
    }

    #[test]
    fn resume_shows_one_image_per_page_in_order() {
        let out = render(&crate::content::fixture_site(), &pages(), "Paul Maxwell").into_string();
        let first = out.find("/resume/page-01.svg").expect("page 1 missing");
        let second = out.find("/resume/page-02.svg").expect("page 2 missing");
        assert!(first < second, "pages rendered out of order");
        assert_eq!(out.matches("<img").count(), 2, "expected exactly one img per page");
    }

    #[test]
    fn resume_keeps_the_pdf_download() {
        let out = render(&crate::content::fixture_site(), &pages(), "text").into_string();
        assert!(out.contains(&format!(r#"href="{RESUME_PDF}""#)), "download link missing");
        assert!(out.contains("download"), "download attribute missing");
    }

    #[test]
    fn resume_embeds_the_extracted_text_for_screen_readers() {
        let out = render(&crate::content::fixture_site(), &pages(), "Aho-Corasick").into_string();
        assert!(out.contains("visually-hidden"), "hidden text container missing");
        assert!(out.contains("Aho-Corasick"), "extracted text not embedded");
    }

    #[test]
    fn resume_without_rendered_pages_still_offers_the_download() {
        let out = render(&crate::content::fixture_site(), &[], "").into_string();
        assert!(out.contains(&format!(r#"href="{RESUME_PDF}""#)), "download link missing");
        assert_eq!(out.matches("<img").count(), 0, "no pages should render no images");
    }

    #[test]
    fn extracted_text_is_escaped_not_injected() {
        let out = render(&crate::content::fixture_site(), &pages(), "a <script>x</script> b").into_string();
        assert!(!out.contains("<script>x</script>"), "raw markup leaked from the PDF text");
        assert!(out.contains("&lt;script&gt;"), "text should be HTML-escaped");
    }
}
