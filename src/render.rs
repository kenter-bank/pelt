/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Pelt rasterizer — SVG → premultiplied-RGBA pixels via usvg + resvg.
//!
//! Supports state variants (extract the matching `data-pelt-state` group) and
//! 9-slice scaling. Output is a plain RGBA buffer the host uploads into its GPU
//! surface (e.g. as a Vello image).

use crate::nine_slice::{composite_9slice, SliceInsets};
use crate::PeltError;

/// Filter SVG to only the matching `data-pelt-state` group (falling back to
/// "default", then to the full SVG).
pub fn filter_svg_state(svg_source: &str, state: &str) -> String {
    if !svg_source.contains("data-pelt-state") {
        return svg_source.to_string();
    }
    if let Some(group) = extract_state_group(svg_source, state) {
        return wrap_in_svg(svg_source, &group);
    }
    if state != "default" {
        if let Some(group) = extract_state_group(svg_source, "default") {
            return wrap_in_svg(svg_source, &group);
        }
    }
    svg_source.to_string()
}

/// Extract the content of `<g data-pelt-state="STATE">…</g>`, matching the
/// closing tag by `<g …>`/`</g>` nesting depth (the group contains nested
/// `pelt-bg`/`pelt-border`/… groups).
fn extract_state_group(svg_source: &str, state: &str) -> Option<String> {
    let attr = format!("data-pelt-state=\"{}\"", state);
    let attr_pos = svg_source.find(&attr)?;
    let content_start = svg_source[attr_pos..].find('>')? + attr_pos + 1;
    let rest = &svg_source[content_start..];

    let mut depth = 1i32;
    let mut i = 0usize;
    while i < rest.len() {
        if rest[i..].starts_with("</g>") {
            depth -= 1;
            if depth == 0 {
                return Some(rest[..i].to_string());
            }
            i += 4;
        } else if rest[i..].starts_with("<g")
            && rest[i + 2..]
                .chars()
                .next()
                .map(|c| c.is_whitespace() || c == '>' || c == '/')
                .unwrap_or(false)
        {
            depth += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    None
}

/// Wrap state-group content in the original root `<svg>`, preserving `<defs>`.
/// Anchors on the `<svg` element (not the first `>`, which may be an `<?xml?>`
/// declaration).
fn wrap_in_svg(svg_source: &str, group_content: &str) -> String {
    let svg_tag = svg_source.find("<svg").unwrap_or(0);
    let svg_open_end = match svg_source[svg_tag..].find('>') {
        Some(pos) => svg_tag + pos + 1,
        None => return svg_source.to_string(),
    };
    let svg_open = &svg_source[..svg_open_end]; // (any <?xml?> prefix) + <svg …>
    let defs = extract_defs(svg_source).unwrap_or_default();
    format!("{}\n{}\n{}\n</svg>", svg_open, defs, group_content)
}

fn extract_defs(svg_source: &str) -> Option<String> {
    let start = svg_source.find("<defs")?;
    let end = svg_source.find("</defs>")? + "</defs>".len();
    Some(svg_source[start..end].to_string())
}

/// Render token-resolved SVG source to a premultiplied-RGBA buffer at
/// `width`×`height`. With `slice_insets`, renders at viewBox size then composites
/// via 9-slice; otherwise stretches to the target size.
pub fn render_svg_to_pixels(
    svg_source: &str,
    width: u32,
    height: u32,
    slice_insets: Option<&SliceInsets>,
) -> Result<Vec<u8>, PeltError> {
    let mut options = usvg::Options::default();
    options.fontdb = crate::fonts::shared();
    let tree =
        usvg::Tree::from_str(svg_source, &options).map_err(|e| PeltError::Svg(e.to_string()))?;
    let svg_size = tree.size();

    if let Some(insets) = slice_insets {
        let vw = svg_size.width().ceil() as u32;
        let vh = svg_size.height().ceil() as u32;
        if vw >= 4
            && vh >= 4
            && width >= 4
            && height >= 4
            && insets.top + insets.bottom < vh as f32
            && insets.left + insets.right < vw as f32
        {
            if let Some(mut src) = resvg::tiny_skia::Pixmap::new(vw, vh) {
                resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut src.as_mut());
                return Ok(composite_9slice(src.data(), vw, vh, width, height, insets));
            }
        }
        // fall through to stretch
    }

    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or(PeltError::Render("pixmap alloc"))?;
    let sx = width as f32 / svg_size.width();
    let sy = height as f32 / svg_size.height();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap.data().to_vec())
}
