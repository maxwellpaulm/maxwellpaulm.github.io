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

/// Splits normalised extracted text on blank lines into paragraph blocks.
/// `.visually-hidden` sets `white-space: nowrap`, which collapses newlines
/// in a single text node — splitting into `<p>` elements is what gives the
/// accessibility tree headings and boundaries to skim by.
fn paragraphs(text: &str) -> Vec<&str> {
    text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()).collect()
}

/// Renders the resume as inline vector pages with a hidden text layer.
///
/// `pages` holds each page in order with its intrinsic size; `text` is the
/// normalised plain-text extraction. With no pages — a local build that has
/// not run `scripts/render-resume.sh` — the page degrades to the download
/// link.
pub fn render(site: &Site, pages: &[ResumePage], text: &str) -> Markup {
    let main = html! {
        h1 { "Resume" }
        p .prose {
            a href=(RESUME_PDF) download { "Download PDF" }
        }

        // alt="" is deliberate: the images are decorative once the hidden
        // text layer below carries the actual content for assistive tech.
        // width/height are the SVG's own declared dimensions — the browser
        // derives the intrinsic aspect ratio from them and reserves the
        // correct box before the file loads, instead of collapsing to zero
        // height and jumping the page.
        @for page in pages {
            img .resume-page src=(page.url) width=(page.width) height=(page.height) alt="";
        }

        @if !text.is_empty() {
            div .visually-hidden {
                h2 { "Resume, text version" }
                @for paragraph in paragraphs(text) {
                    p { (paragraph) }
                }
            }
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
        let out = render(&crate::content::fixture_site(), &pages(), "Paul Maxwell").into_string();
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
        let out = render(&crate::content::fixture_site(), &pages, "text").into_string();
        assert!(
            out.contains(r#"src="/resume/page-01.svg" width="612" height="792""#),
            "img missing its parsed width/height attributes: {out}"
        );
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
    fn resume_hidden_text_is_split_into_paragraphs_behind_a_heading() {
        let text = "Paul Maxwell\nWashington, DC\n\nEXPERIENCE\nP-1 AI, Remote\n\nEDUCATION\nGeorgia Tech";
        let out = render(&crate::content::fixture_site(), &pages(), text).into_string();
        assert!(
            out.contains("<h2>Resume, text version</h2>"),
            "missing the navigable heading: {out}"
        );
        assert_eq!(
            out.matches("<p>").count(),
            3,
            "expected one <p> per blank-line-separated block: {out}"
        );
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
