use crate::content::Work;
use maud::{html, Markup};

pub fn work_list(items: &[Work]) -> Markup {
    html! {
        @for item in items {
            div .item {
                div {
                    h3 { (item.title) " " span .org { "· " (item.org) } }
                    p { (item.summary) }
                }
                div .mono .year { (item.year) }
            }
        }
    }
}

/// Same as `work_list`, plus a longer write-up paragraph per item when
/// `detail` is set. Used on the Projects page only — the homepage keeps
/// the short cards from `work_list`.
pub fn work_list_detailed(items: &[Work]) -> Markup {
    html! {
        @for item in items {
            div .item {
                div {
                    h3 { (item.title) " " span .org { "· " (item.org) } }
                    p { (item.summary) }
                    @if !item.detail.is_empty() {
                        p .work-detail { (item.detail) }
                    }
                }
                div .mono .year { (item.year) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Work> {
        vec![Work {
            title: "Zero-Trust Agent Gateway".into(),
            org: "P-1 AI".into(),
            year: "2025".into(),
            summary: "Phantom-token pattern.".into(),
            detail: String::new(),
        }]
    }

    #[test]
    fn renders_one_item_per_entry_with_org_and_year() {
        let out = work_list(&items()).into_string();
        assert!(out.contains("Zero-Trust Agent Gateway"));
        assert!(out.contains(r#"class="org""#));
        assert!(out.contains("P-1 AI"));
        assert!(out.contains("2025"));
        assert!(out.contains("Phantom-token pattern."));
        assert_eq!(out.matches(r#"class="item""#).count(), 1);
    }

    #[test]
    fn empty_input_renders_nothing_rather_than_an_empty_shell() {
        assert_eq!(work_list(&[]).into_string(), "");
    }

    #[test]
    fn detailed_variant_renders_the_detail_paragraph() {
        let mut item = items().remove(0);
        item.detail = "The gateway holds every real credential.".into();
        let out = work_list_detailed(&[item]).into_string();
        assert!(out.contains(r#"class="work-detail""#));
        assert!(out.contains("The gateway holds every real credential."));
    }

    #[test]
    fn detailed_variant_omits_the_paragraph_when_detail_is_empty() {
        let out = work_list_detailed(&items()).into_string();
        assert!(!out.contains("work-detail"));
    }
}
