use super::{GraphicsKind, GraphicsObject, needs_raster_background};

#[test]
fn graphics_require_a_raster_background() {
    let graphics = vec![GraphicsObject {
        paint_order: 0,
        kind: GraphicsKind::Path,
        bounds: None,
        active: Some(true),
        children: Vec::new(),
    }];
    assert!(needs_raster_background(&[], &graphics));
}

#[test]
fn text_only_pages_do_not_require_a_raster_background() {
    assert!(!needs_raster_background(&[], &[]));
}
