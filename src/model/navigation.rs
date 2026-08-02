use serde::Serialize;

use super::geometry::Rect;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Link {
    pub bounds: Rect,
    pub target: LinkTarget,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum LinkTarget {
    Uri(String),
    LocalDestination,
    RemoteDestination,
    Unknown,
}
