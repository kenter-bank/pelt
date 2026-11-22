//! Render the bundled sample pelts end-to-end (parse → tokens → state → 9-slice
//! → RGBA), the Milestone-1 proof that the engine works standalone.

use pelt::{Pelt, ScaleMode};

#[test]
fn renders_neon_button_default_and_hover() {
    let mut p = Pelt::load("samples/neon-button.svg").expect("load");
    assert_eq!(p.scale_mode(), ScaleMode::NineSlice);

    // Default state, 9-slice scaled to a wide button.
    let img = p.render(360, 72).expect("render default");
    assert_eq!((img.width, img.height), (360, 72));
    assert_eq!(img.rgba.len(), 360 * 72 * 4);
    assert!(img.rgba.iter().any(|&b| b != 0), "rendered nothing");

    // Hover state renders too (different pixels than default is not asserted,
    // just that the state path works).
    p.set_state("hover");
    let hov = p.render(360, 72).expect("render hover");
    assert!(hov.rgba.iter().any(|&b| b != 0));
}

#[test]
fn token_override_changes_pixels() {
    let base = Pelt::load("samples/neon-button.svg")
        .unwrap()
        .render(240, 60)
        .unwrap();

    let mut themed = Pelt::load("samples/neon-button.svg").unwrap();
    themed.set_token("surface", "#00ff88");
    themed.set_token("accent", "#00ff88");
    let out = themed.render(240, 60).unwrap();

    assert_ne!(base.rgba, out.rgba, "token override had no visual effect");
}

#[test]
fn text_renders_with_system_fonts() {
    // A bare `<text>` element only produces pixels if a font database is wired
    // into the render options (see src/fonts.rs). Empty output = fonts not loaded.
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 60">
        <text x="10" y="40" font-family="Arial, sans-serif" font-size="32" fill="#ffffff">Kenter</text>
      </svg>"##;
    let img = Pelt::from_str(svg).unwrap().render(200, 60).unwrap();
    let opaque = img.rgba.chunks_exact(4).filter(|px| px[3] > 0).count();
    assert!(opaque > 50, "text produced {opaque} opaque pixels — fontdb not wired?");
}

#[test]
fn glass_card_renders() {
    let img = Pelt::load("samples/glass-card.svg")
        .unwrap()
        .render(400, 260)
        .unwrap();
    assert!(img.rgba.iter().any(|&b| b != 0));
}
