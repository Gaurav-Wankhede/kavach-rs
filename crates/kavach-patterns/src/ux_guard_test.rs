use super::*;

#[test]
fn blocks_arbitrary_px() {
    assert!(check("src/C.tsx", "className=\"p-[12px]\"").is_some());
}

#[test]
fn allows_scale_spacing() {
    assert!(check("src/C.tsx", "className=\"p-4 m-6\"").is_none());
}

#[test]
fn allows_layout_in_layouts() {
    assert!(check("src/layouts/Base.astro", "<html>").is_none());
}

#[test]
fn blocks_layout_in_component() {
    assert!(check("src/components/Card.astro", "<html>").is_some());
}

#[test]
fn skips_rust() {
    assert!(check("src/main.rs", "p-[12px]").is_none());
}
