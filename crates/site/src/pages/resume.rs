use crate::components::shell;
use crate::content::Site;
use crate::route::Route;
use maud::{html, Markup};

/// Published location of the PDF fetched from the private release.
pub const RESUME_PDF: &str = "/assets/paul_maxwell_resume.pdf";

/// One rendered resume page: its rooted URL plus the intrinsic dimensions
/// declared on the source SVG's root element, so the `<img>` can reserve
/// the correct box before the file loads.
#[derive(Debug)]
pub struct ResumePage {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

/// Renders the resume as inline vector pages.
///
/// `pages` holds each page in order with its intrinsic size. With no pages —
/// a local build that has not run `scripts/render-resume.sh` — the page
/// degrades to the download link.
///
/// There is no machine-readable text layer: PDF text extraction produced
/// unstructured, poorly-segmented output not worth shipping. Each image
/// instead carries a descriptive alt naming its page number, so assistive
/// tech at least announces the resume's presence and the adjacent download
/// link.
pub fn render(site: &Site, pages: &[ResumePage]) -> Markup {
    let total = pages.len();
    let main = html! {
        h1 { "Resume" }
        p .prose {
            a href=(RESUME_PDF) download { "Download PDF" }
        }

        // width/height are the SVG's own declared dimensions — the browser
        // derives the intrinsic aspect ratio from them and reserves the
        // correct box before the file loads, instead of collapsing to zero
        // height and jumping the page.
        @for (i, page) in pages.iter().enumerate() {
            img .resume-page src=(page.url) width=(page.width) height=(page.height)
                alt=(format!("Resume, page {} of {total}", i + 1));
        }
    };
    shell::layout(site, Route::Resume, "Resume", main)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages() -> Vec<ResumePage> {
        vec![
            ResumePage { url: "/resume/page-01.svg".to_string(), width: 612, height: 792 },
            ResumePage { url: "/resume/page-02.svg".to_string(), width: 612, height: 792 },
        ]
    }

    #[test]
    fn resume_shows_one_image_per_page_in_order() {
        let out = render(&crate::content::fixture_site(), &pages()).into_string();
        let first = out.find("/resume/page-01.svg").expect("page 1 missing");
        let second = out.find("/resume/page-02.svg").expect("page 2 missing");
        assert!(first < second, "pages rendered out of order");
        assert_eq!(out.matches("<img").count(), 2, "expected exactly one img per page");
    }

    #[test]
    fn resume_image_carries_its_parsed_intrinsic_dimensions() {
        let mut pages = pages();
        pages[0].width = 612;
        pages[0].height = 792;
        let out = render(&crate::content::fixture_site(), &pages).into_string();
        assert!(
            out.contains(r#"src="/resume/page-01.svg" width="612" height="792""#),
            "img missing its parsed width/height attributes: {out}"
        );
    }

    #[test]
    fn resume_keeps_the_pdf_download() {
        let out = render(&crate::content::fixture_site(), &pages()).into_string();
        assert!(out.contains(&format!(r#"href="{RESUME_PDF}""#)), "download link missing");
        assert!(out.contains("download"), "download attribute missing");
    }

    #[test]
    fn resume_images_carry_a_meaningful_alt_naming_the_page_number() {
        let out = render(&crate::content::fixture_site(), &pages()).into_string();
        assert!(out.contains(r#"alt="Resume, page 1 of 2""#), "page 1 alt missing or empty: {out}");
        assert!(out.contains(r#"alt="Resume, page 2 of 2""#), "page 2 alt missing or empty: {out}");
    }

    #[test]
    fn resume_without_rendered_pages_still_offers_the_download() {
        let out = render(&crate::content::fixture_site(), &[]).into_string();
        assert!(out.contains(&format!(r#"href="{RESUME_PDF}""#)), "download link missing");
        assert_eq!(out.matches("<img").count(), 0, "no pages should render no images");
    }
}
