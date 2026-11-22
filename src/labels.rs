//! Text styles a skin declares for its host to fill in.
//!
//! A skin is the design, and the words on an element are part of how it
//! looks. So rather than the host deciding size, colour, weight and position
//! in code, the skin carries a sample of each piece of text it expects, as a
//! real SVG `<text>` an artist can see and edit in Inkscape:
//!
//! ```xml
//! <g inkscape:groupmode="layer" inkscape:label="text">
//!   <text data-pelt-label="amount" x="16" y="31" font-family="Segoe UI"
//!         font-size="15" font-weight="700" fill="#ffffff">$25.00</text>
//! </g>
//! ```
//!
//! The host asks for `amount`, gets its position and style, and draws its own
//! words there. The sample text is never rasterised as part of the skin: it
//! lives outside the `data-pelt-state` groups, which are all a render draws.
//!
//! Inkscape rewrites `<text>` on save: properties move into `style="..."`,
//! weights become `bold`/`normal`, sizes gain `px`, and the position often
//! moves onto a `<tspan>` child. All of those are read.

/// Where a piece of text sits and how it looks, in the skin's viewBox units.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub name: String,
    /// The anchor point: left edge, centre or right edge per `anchor`.
    pub x: f32,
    /// The baseline.
    pub y: f32,
    pub size: f32,
    pub weight: u32,
    /// As written in the file: `#rrggbb`, `#rgb` or a named colour.
    pub fill: String,
    pub anchor: TextAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

/// Every `<text data-pelt-label="...">` in `svg`, in document order.
pub fn text_styles(svg: &str) -> Vec<TextStyle> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(i) = rest.find("<text") {
        let after = &rest[i + 5..];
        // `<textPath` and friends are not text elements.
        if !after.starts_with(|c: char| c.is_whitespace() || c == '>') {
            rest = after;
            continue;
        }
        let Some(open_end) = after.find('>') else { break };
        let open = &after[..open_end];
        let close = after.find("</text>").unwrap_or(after.len());
        let body = &after[open_end.min(close)..close];
        rest = &after[close.min(after.len())..];

        let Some(name) = attr(open, "data-pelt-label") else { continue };
        let style = attr(open, "style").unwrap_or_default();
        let prop = |key: &str| attr(open, key).or_else(|| style_prop(&style, key));

        // Position: on the element, or on its first tspan (Inkscape).
        let tspan = body
            .find("<tspan")
            .map(|t| &body[t..body[t..].find('>').map(|e| t + e).unwrap_or(body.len())]);
        let coord = |k: &str| {
            attr(open, k)
                .or_else(|| tspan.and_then(|t| attr(t, k)))
                .and_then(|v| number(&v))
        };

        out.push(TextStyle {
            name,
            x: coord("x").unwrap_or(0.0),
            y: coord("y").unwrap_or(0.0),
            size: prop("font-size").and_then(|v| number(&v)).unwrap_or(12.0),
            weight: prop("font-weight").map(|v| weight(&v)).unwrap_or(400),
            fill: prop("fill").unwrap_or_else(|| "#000000".into()),
            anchor: match prop("text-anchor").as_deref() {
                Some("middle") => TextAnchor::Middle,
                Some("end") => TextAnchor::End,
                _ => TextAnchor::Start,
            },
        });
    }
    out
}

/// The `viewBox` of the root `<svg>`, as `(min_x, min_y, width, height)`.
pub fn view_box(svg: &str) -> Option<(f32, f32, f32, f32)> {
    let i = svg.find("<svg")?;
    let tag = &svg[i..i + svg[i..].find('>')?];
    let v: Vec<f32> = attr(tag, "viewBox")?
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    (v.len() == 4).then(|| (v[0], v[1], v[2], v[3]))
}

/// An attribute's value, matching the whole name: `x` is not found in `rx`.
fn attr(tag: &str, name: &str) -> Option<String> {
    let pat = format!("{name}=");
    let mut from = 0;
    while let Some(off) = tag[from..].find(&pat) {
        let at = from + off;
        let boundary = at == 0
            || tag[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace());
        let after = &tag[at + pat.len()..];
        if boundary {
            let quote = after.chars().next()?;
            if quote == '"' || quote == '\'' {
                let val = &after[1..];
                return val.find(quote).map(|e| val[..e].to_string());
            }
        }
        from = at + pat.len();
    }
    None
}

fn style_prop(style: &str, key: &str) -> Option<String> {
    style.split(';').find_map(|decl| {
        let (k, v) = decl.split_once(':')?;
        (k.trim() == key).then(|| v.trim().to_string())
    })
}

/// `15`, `15px`, `15.5pt` -> the number. Units other than px are taken as px.
fn number(v: &str) -> Option<f32> {
    let n: String = v
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    n.parse().ok()
}

fn weight(v: &str) -> u32 {
    match v.trim() {
        "bold" | "bolder" => 700,
        "normal" | "lighter" => 400,
        w => w.parse().unwrap_or(400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_attributes() {
        let svg = r##"<svg viewBox="0 0 320 52"><g>
            <text data-pelt-label="amount" x="16" y="31.1" font-size="15"
                  font-weight="700" fill="#ffffff">$25</text>
            <text data-pelt-label="status" x="304" y="26" font-size="12"
                  font-weight="500" fill="#7f869a" text-anchor="end">live</text>
            <text x="1" y="1">not a label</text>
        </g></svg>"##;
        let t = text_styles(svg);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].name, "amount");
        assert_eq!((t[0].x, t[0].y, t[0].size, t[0].weight), (16.0, 31.1, 15.0, 700));
        assert_eq!(t[0].fill, "#ffffff");
        assert_eq!(t[1].anchor, TextAnchor::End);
        assert_eq!(view_box(svg), Some((0.0, 0.0, 320.0, 52.0)));
    }

    /// What Inkscape writes after an edit: style properties, `bold`, `px`,
    /// and the position on a tspan.
    #[test]
    fn reads_what_inkscape_saves() {
        let svg = r##"<svg viewBox="0 0 320 52"><text xml:space="preserve"
            style="font-size:15px;font-weight:bold;font-family:'Segoe UI';fill:#ffffff;text-anchor:middle"
            data-pelt-label="amount"><tspan sodipodi:role="line" x="160" y="31">$25</tspan></text></svg>"##;
        let t = &text_styles(svg)[0];
        assert_eq!((t.x, t.y, t.size, t.weight), (160.0, 31.0, 15.0, 700));
        assert_eq!(t.anchor, TextAnchor::Middle);
        assert_eq!(t.fill, "#ffffff");
    }

    #[test]
    fn x_is_not_found_inside_rx() {
        assert_eq!(attr(r#"<rect rx="8" x="3""#, "x").as_deref(), Some("3"));
        assert_eq!(attr(r#"<rect rx="8""#, "x"), None);
    }
}
