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

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Work> {
        vec![Work {
            title: "Zero-Trust Agent Gateway".into(),
            org: "P-1 AI".into(),
            year: "2025".into(),
            summary: "Phantom-token pattern.".into(),
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
}
