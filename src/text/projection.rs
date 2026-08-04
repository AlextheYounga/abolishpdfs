use crate::model::AffineTransform;

/// Tolerance used when deciding whether a transform is the identity rotation.
pub const EPSILON: f32 = 1e-3;

/// Returns true when the 2x2 part of `matrix` is the identity rotation,
/// meaning the text runs horizontally without rotation, shear, or anisotropic
/// scaling. Translation (`e`, `f`) is ignored: it is carried by the glyph's
/// origin instead.
pub fn is_identity(matrix: &AffineTransform) -> bool {
    (matrix.a - 1.0).abs() < EPSILON
        && matrix.b.abs() < EPSILON
        && matrix.c.abs() < EPSILON
        && (matrix.d - 1.0).abs() < EPSILON
}

/// A PDF text matrix rescaled for CSS: a `font-size` multiplier (`scale`) plus
/// a rotation matrix whose vertical axis has unit length.
///
/// This mirrors pdf2htmlEX's `new_draw_text_scale` normalization
/// (`HTMLRenderer/state.cc`): the font size absorbs the full vertical scale and
/// the remaining matrix carries only the rotation and any residual anisotropy,
/// so writing `font-size` plus a CSS transform reproduces the PDF geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection {
    pub scale: f32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
}

/// Normalizes `matrix` for CSS output. Returns `None` when the matrix has no
/// usable vertical scale (both `c` and `d` zero), in which case callers should
/// treat the text as unrotated with the raw matrix.
pub fn project(matrix: &AffineTransform) -> Option<Projection> {
    let scale = matrix.c.hypot(matrix.d);
    if scale <= EPSILON {
        return None;
    }
    Some(Projection { scale, a: matrix.a / scale, b: matrix.b / scale, c: matrix.c / scale, d: matrix.d / scale })
}

impl Projection {
    /// Renders the projection as a CSS `transform:matrix(...)` with the y axis
    /// flipped to CSS orientation and no translation, matching pdf2htmlEX's
    /// `TransformMatrixManager::dump_value`.
    pub fn to_css(&self) -> String {
        format!(
            "transform:matrix({},{},{},{},0,0);",
            css_number(self.a),
            css_number(-self.b),
            css_number(-self.c),
            css_number(self.d)
        )
    }
}

/// Formats a PDF coordinate as a CSS length, trimming trailing zeros.
pub fn css_number(value: f32) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    format!("{value:.4}").trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(a: f32, b: f32, c: f32, d: f32) -> AffineTransform {
        AffineTransform { a, b, c, d, e: 0.0, f: 0.0 }
    }

    #[test]
    fn identity_matrix_is_detected() {
        assert!(is_identity(&matrix(1.0, 0.0, 0.0, 1.0)));
        assert!(is_identity(&AffineTransform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 72.0, f: 720.0 }));
        assert!(!is_identity(&matrix(0.0, 1.0, -1.0, 0.0)));
    }

    #[test]
    fn quarter_turn_projection_keeps_font_scale() {
        let projection = project(&matrix(0.0, 1.0, -1.0, 0.0)).unwrap();
        assert!((projection.scale - 1.0).abs() < EPSILON);
        assert!((projection.a - 0.0).abs() < EPSILON);
        assert!((projection.b - 1.0).abs() < EPSILON);
        assert!((projection.c - -1.0).abs() < EPSILON);
        assert!((projection.d - 0.0).abs() < EPSILON);
    }

    #[test]
    fn y_axis_is_flipped_in_css_matrix() {
        let projection = project(&matrix(0.0, 1.0, -1.0, 0.0)).unwrap();
        assert_eq!(projection.to_css(), "transform:matrix(0,-1,1,0,0,0);");
    }

    #[test]
    fn vertical_scale_is_absorbed_into_font_size() {
        let projection = project(&matrix(1.0, 0.0, 0.0, 2.0)).unwrap();
        assert!((projection.scale - 2.0).abs() < EPSILON);
        assert_eq!(projection.to_css(), "transform:matrix(0.5,0,0,1,0,0);");
    }

    #[test]
    fn degenerate_matrix_has_no_projection() {
        assert!(project(&matrix(1.0, 0.0, 0.0, 0.0)).is_none());
    }
}
