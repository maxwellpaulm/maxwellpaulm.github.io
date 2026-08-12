#[derive(Clone, Copy)]
pub struct Palette {
    pub paper: &'static str,
    pub surface: &'static str,
    pub ink: &'static str,
    pub muted: &'static str,
    pub rule: &'static str,
    pub accent: &'static str,
}

pub const LIGHT: Palette = Palette {
    paper: "#FBFAF8",
    surface: "#FFFFFF",
    ink: "#14161A",
    muted: "#6E7076",
    rule: "#E5E2DC",
    accent: "#A8431E",
};

pub const DARK: Palette = Palette {
    paper: "#0E0F11",
    surface: "#16181B",
    ink: "#E9E7E2",
    muted: "#94989F",
    rule: "#25282D",
    accent: "#E0764A",
};

#[cfg(test)]
fn channel_luminance(byte: u8) -> f64 {
    let c = f64::from(byte) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
fn relative_luminance(hex: &str) -> f64 {
    let h = hex.trim_start_matches('#');
    let parse = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).expect("valid hex colour");
    0.2126 * channel_luminance(parse(0))
        + 0.7152 * channel_luminance(parse(2))
        + 0.0722 * channel_luminance(parse(4))
}

/// WCAG 2.1 contrast ratio between two hex colours, from 1.0 to 21.0.
///
/// Exists purely to enforce the WCAG invariant in tests; no build-time
/// caller, so it lives only in the test target.
#[cfg(test)]
pub fn contrast_ratio(fg: &str, bg: &str) -> f64 {
    let (a, b) = (relative_luminance(fg), relative_luminance(bg));
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

fn tokens(p: Palette) -> String {
    format!(
        "  --paper: {};\n  --surface: {};\n  --ink: {};\n  --muted: {};\n  --rule: {};\n  --accent: {};\n",
        p.paper, p.surface, p.ink, p.muted, p.rule, p.accent
    )
}

/// The complete stylesheet, emitted from the tokens above so light and dark
/// stay one system rather than two hand-maintained sheets.
pub fn stylesheet() -> String {
    format!(
        r#"@font-face {{
  font-family: "Inter";
  src: url("/fonts/InterVariable.woff2") format("woff2");
  font-weight: 100 900;
  font-display: swap;
}}
@font-face {{
  font-family: "JetBrains Mono";
  src: url("/fonts/JetBrainsMono.woff2") format("woff2");
  font-weight: 400;
  font-display: swap;
}}
:root {{
{light}
  --space: 8px;
  --measure: 62ch;
  --font-sans: "Inter", "Helvetica Neue", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, Menlo, monospace;
  --motion: 180ms;
}}

@media (prefers-color-scheme: dark) {{
  :root {{
{dark}
  }}
}}

*, *::before, *::after {{ box-sizing: border-box; }}

body {{
  margin: 0;
  background: var(--paper);
  color: var(--ink);
  font-family: var(--font-sans);
  font-size: 16px;
  line-height: 1.6;
  font-weight: 350;
  -webkit-font-smoothing: antialiased;
}}

a {{ color: var(--accent); text-decoration: none; }}
a:hover {{ text-decoration: underline; }}

:focus-visible {{
  outline: 2px solid var(--accent);
  outline-offset: 3px;
  border-radius: 2px;
}}

.mono {{
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--muted);
}}

/* Composition A: persistent left rail, content off-centre. */
.layout {{ display: grid; grid-template-columns: 132px 1fr; min-height: 100vh; }}
.rail {{
  border-right: 1px solid var(--rule);
  padding: calc(var(--space) * 4) calc(var(--space) * 2.5);
  display: flex;
  flex-direction: column;
  gap: calc(var(--space) * 3.5);
}}
.rail nav {{ display: flex; flex-direction: column; gap: 9px; }}
.rail nav a {{ font-size: 13px; color: var(--muted); transition: color var(--motion) ease; }}
.rail nav a[aria-current="page"] {{ color: var(--ink); font-weight: 500; }}
.rail nav a[aria-current="page"]::before {{ content: "— "; color: var(--accent); }}
.rail-foot {{ margin-top: auto; }}

main {{ padding: calc(var(--space) * 6.5) calc(var(--space) * 7) calc(var(--space) * 5); }}
h1 {{ font-size: 54px; line-height: 1.02; letter-spacing: -0.035em; font-weight: 600; margin: 0 0 calc(var(--space) * 3.25); }}
h2 {{ font-size: 24px; letter-spacing: -0.02em; font-weight: 600; }}
.lede {{ font-size: 17px; line-height: 1.62; max-width: var(--measure); margin: 0 0 calc(var(--space) * 1.75); }}
.prose {{ font-size: 14.5px; line-height: 1.65; max-width: var(--measure); color: var(--muted); }}

.section-head {{
  display: flex; justify-content: space-between; align-items: baseline;
  padding-bottom: 11px; border-bottom: 1px solid var(--ink);
  margin: calc(var(--space) * 5.5) 0 0;
}}
.section-head .mono {{ color: var(--ink); }}

.item {{
  display: grid; grid-template-columns: 1fr 74px; gap: calc(var(--space) * 2.25);
  padding: calc(var(--space) * 2.125) 0; border-bottom: 1px solid var(--rule);
  align-items: start;
}}
.item h3 {{ margin: 0 0 5px; font-size: 15px; font-weight: 500; letter-spacing: -0.01em; }}
.item p {{ margin: 0; font-size: 13.5px; line-height: 1.55; color: var(--muted); max-width: var(--measure); }}
.item .year {{ text-align: right; }}
.org {{ color: var(--accent); }}

/* The rail becomes a top bar before the grid gets cramped. */
@media (max-width: 640px) {{
  .layout {{ grid-template-columns: 1fr; }}
  .rail {{
    border-right: 0; border-bottom: 1px solid var(--rule);
    flex-direction: row; align-items: center; justify-content: space-between;
    gap: calc(var(--space) * 2); padding: calc(var(--space) * 2);
  }}
  .rail nav {{ flex-direction: row; gap: calc(var(--space) * 2); flex-wrap: wrap; }}
  .rail-foot {{ margin-top: 0; }}
  main {{ padding: calc(var(--space) * 4) calc(var(--space) * 2.5); }}
  h1 {{ font-size: 38px; }}
  .item {{ grid-template-columns: 1fr; gap: 4px; }}
  .item .year {{ text-align: left; }}
}}

@media (prefers-reduced-motion: reduce) {{
  *, *::before, *::after {{
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }}
}}
"#,
        light = tokens(LIGHT),
        dark = tokens(DARK),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_ratio_matches_known_values() {
        // Black on white is the reference maximum, 21:1.
        assert!((contrast_ratio("#000000", "#FFFFFF") - 21.0).abs() < 0.01);
        assert!((contrast_ratio("#FFFFFF", "#FFFFFF") - 1.0).abs() < 0.01);
    }

    #[test]
    fn every_token_pair_meets_wcag_aa() {
        for (name, p) in [("light", LIGHT), ("dark", DARK)] {
            for (label, fg) in [("ink", p.ink), ("muted", p.muted), ("accent", p.accent)] {
                let ratio = contrast_ratio(fg, p.paper);
                assert!(
                    ratio >= 4.5,
                    "{name}/{label} on paper is {ratio:.2}:1, below WCAG AA 4.5:1"
                );
            }
        }
    }

    #[test]
    fn stylesheet_defines_both_themes_and_respects_reduced_motion() {
        let css = stylesheet();
        assert!(css.contains("--paper: #FBFAF8"), "light tokens missing");
        assert!(css.contains("--paper: #0E0F11"), "dark tokens missing");
        assert!(css.contains("prefers-color-scheme: dark"));
        assert!(css.contains("prefers-reduced-motion: reduce"));
        assert!(css.contains(":focus-visible"), "focus ring missing");
    }
}
