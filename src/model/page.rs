use serde::Serialize;

use super::geometry::{Rect, Size};
use super::navigation::Link;
use super::text::TextObject;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageModel {
    pub number: usize,
    pub size: Size,
    pub crop_box: Rect,
    pub text_objects: Vec<TextObject>,
    pub graphics: Vec<GraphicsObject>,
    pub links: Vec<Link>,
    pub background: Option<RasterBackground>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RasterBackground {
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GraphicsObject {
    pub paint_order: usize,
    pub kind: GraphicsKind,
    pub bounds: Option<Rect>,
    pub active: Option<bool>,
    pub children: Vec<GraphicsObject>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GraphicsKind {
    Path,
    Image,
    Shading,
    Form,
    Unsupported,
}
