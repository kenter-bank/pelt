/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! # pelt
//!
//! **SVG "pelt" skins as a UI appearance layer.** A pelt is a full SVG canvas
//! stretched over an element's bounding rect — the *look*, independent of layout.
//! Where CSS uses property/value pairs, a pelt is drawn (in Inkscape or any SVG
//! editor) and carries a tiny `pelt:` convention set:
//!
//! - **theme tokens** — `var(--pelt-<name>, fallback)` substituted before render,
//! - **state variants** — `data-pelt-state="default|hover|active|focus|…"` groups,
//! - **9-slice scaling** — `pelt:scale="9-slice"` + `<pelt:slices …>`,
//! - **animation** — driven per-frame by the host (see `animation`/`transitions`).
//!
//! Rendering is CPU (usvg + resvg) to a premultiplied-RGBA buffer; the host
//! composites that into its GPU surface (e.g. a Vello image) for a clean,
//! controllable per-frame loop.
//!
//! ```no_run
//! let mut p = pelt::Pelt::load("skins/button-primary.svg").unwrap();
//! p.set_token("accent", "#39ff14");
//! p.set_state("hover");
//! let img = p.render(240, 60).unwrap(); // img.rgba: premultiplied RGBA
//! ```

pub mod fonts;
pub mod labels;
pub mod nine_slice;
pub mod render;
pub mod token_resolver;

use std::collections::HashMap;
use std::path::Path;

pub use nine_slice::SliceInsets;

/// How a pelt scales to fill its element rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleMode {
    /// Stretch the whole SVG to the target (default).
    Stretch,
    /// Keep corners fixed; stretch edges/center (`pelt:scale="9-slice"`).
    NineSlice,
}

/// A rendered pelt image: premultiplied RGBA, row-major, `width`×`height`.
#[derive(Debug, Clone)]
pub struct PeltImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl PeltImage {
    /// Encode to PNG bytes (for tests / previews).
    pub fn encode_png(&self) -> Result<Vec<u8>, PeltError> {
        let size = resvg::tiny_skia::IntSize::from_wh(self.width, self.height)
            .ok_or(PeltError::Render("bad size"))?;
        let pixmap = resvg::tiny_skia::Pixmap::from_vec(self.rgba.clone(), size)
            .ok_or(PeltError::Render("pixmap from rgba"))?;
        pixmap
            .encode_png()
            .map_err(|e| PeltError::RenderOwned(e.to_string()))
    }

    /// Encode and write a PNG file.
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), PeltError> {
        std::fs::write(path, self.encode_png()?).map_err(|e| PeltError::Io(e.to_string()))
    }
}

/// A loaded pelt skin: SVG source + parsed conventions + current tokens/state.
#[derive(Debug, Clone)]
pub struct Pelt {
    source: String,
    scale: ScaleMode,
    slices: Option<SliceInsets>,
    tokens: HashMap<String, String>,
    state: String,
}

impl Pelt {
    /// Load a pelt from an SVG file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PeltError> {
        let src = std::fs::read_to_string(path).map_err(|e| PeltError::Io(e.to_string()))?;
        Self::from_str(&src)
    }

    /// Parse a pelt from SVG source.
    pub fn from_str(svg: &str) -> Result<Self, PeltError> {
        let scale = if attr(svg, "pelt:scale").as_deref() == Some("9-slice") {
            ScaleMode::NineSlice
        } else {
            ScaleMode::Stretch
        };
        let slices = parse_slices(svg);
        Ok(Self {
            source: svg.to_string(),
            scale,
            slices,
            tokens: HashMap::new(),
            state: "default".to_string(),
        })
    }

    /// Override a theme token (`--pelt-<name>`).
    pub fn set_token(&mut self, name: &str, value: impl Into<String>) {
        self.tokens.insert(name.to_string(), value.into());
    }

    /// Select the state variant to render.
    pub fn set_state(&mut self, state: &str) {
        self.state = state.to_string();
    }

    /// Current state.
    pub fn state(&self) -> &str {
        &self.state
    }

    /// The text styles this skin declares (`labels.rs`).
    pub fn text_styles(&self) -> Vec<labels::TextStyle> {
        labels::text_styles(&self.source)
    }

    /// The skin's own canvas, `(min_x, min_y, width, height)`.
    pub fn view_box(&self) -> Option<(f32, f32, f32, f32)> {
        labels::view_box(&self.source)
    }

    /// Scale mode declared by the pelt.
    pub fn scale_mode(&self) -> ScaleMode {
        self.scale
    }

    /// Render the current state at `width`×`height` → premultiplied RGBA.
    pub fn render(&self, width: u32, height: u32) -> Result<PeltImage, PeltError> {
        let filtered = render::filter_svg_state(&self.source, &self.state);
        let resolved = token_resolver::resolve_tokens(&filtered, &self.tokens);
        let insets = match self.scale {
            ScaleMode::NineSlice => self.slices.as_ref(),
            ScaleMode::Stretch => None,
        };
        let rgba = render::render_svg_to_pixels(&resolved, width, height, insets)?;
        Ok(PeltImage { width, height, rgba })
    }
}

/// Errors from loading/rendering a pelt.
#[derive(Debug, thiserror::Error)]
pub enum PeltError {
    #[error("i/o error: {0}")]
    Io(String),
    #[error("svg parse error: {0}")]
    Svg(String),
    #[error("render error: {0}")]
    Render(&'static str),
    #[error("render error: {0}")]
    RenderOwned(String),
}

// -- lightweight schema parsing (no XML dep; the values are simple attrs) --

/// Read a `name="value"` attribute value from the SVG source.
fn attr(svg: &str, name: &str) -> Option<String> {
    let key = format!("{}=\"", name);
    let start = svg.find(&key)? + key.len();
    let end = svg[start..].find('"')? + start;
    Some(svg[start..end].to_string())
}

/// Parse `<pelt:slices top=".." right=".." bottom=".." left="..">`.
fn parse_slices(svg: &str) -> Option<SliceInsets> {
    let tag_start = svg.find("<pelt:slices")?;
    let tag = &svg[tag_start..svg[tag_start..].find('>').map(|e| tag_start + e)?];
    let num = |k: &str| -> f32 {
        attr(tag, k).and_then(|v| v.parse().ok()).unwrap_or(0.0)
    };
    Some(SliceInsets {
        top: num("top"),
        right: num("right"),
        bottom: num("bottom"),
        left: num("left"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scale_and_slices() {
        let svg = r#"<svg xmlns:pelt="x" pelt:scale="9-slice">
          <pelt:slices top="16" right="20" bottom="16" left="20"/></svg>"#;
        let p = Pelt::from_str(svg).unwrap();
        assert_eq!(p.scale_mode(), ScaleMode::NineSlice);
        let s = p.slices.unwrap();
        assert_eq!((s.top, s.right, s.bottom, s.left), (16.0, 20.0, 16.0, 20.0));
    }
}
