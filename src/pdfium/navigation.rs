use pdfium_render::prelude::*;

use crate::model::{LinkTarget, OutlineItem};

pub(crate) fn link_target(link: &PdfLink<'_>) -> LinkTarget {
    let Some(action) = link.action() else {
        return link
            .destination()
            .and_then(|destination| destination.page_index().ok())
            .and_then(|index| usize::try_from(index).ok())
            .map_or(LinkTarget::Unknown, |index| LinkTarget::LocalDestination(index + 1));
    };
    if let Some(uri) = action.as_uri_action().and_then(|uri| uri.uri().ok()) {
        return LinkTarget::Uri(uri);
    }
    if let Some(local) = action.as_local_destination_action() {
        local
            .destination()
            .ok()
            .and_then(|destination| destination.page_index().ok())
            .and_then(|index| usize::try_from(index).ok())
            .map_or(LinkTarget::Unknown, |index| LinkTarget::LocalDestination(index + 1))
    } else if action.as_remote_destination_action().is_some() {
        LinkTarget::RemoteDestination
    } else {
        LinkTarget::Unknown
    }
}

pub(crate) fn extract_outlines(document: &PdfDocument<'_>) -> Vec<OutlineItem> {
    let mut outlines = Vec::new();
    let mut bookmark = document.bookmarks().root();
    while let Some(current) = bookmark {
        outlines.push(outline_item(&current));
        bookmark = current.next_sibling();
    }
    outlines
}

fn outline_item(bookmark: &PdfBookmark<'_>) -> OutlineItem {
    let mut children = Vec::new();
    let mut child = bookmark.first_child();
    while let Some(current) = child {
        children.push(outline_item(&current));
        child = current.next_sibling();
    }
    let target_page = bookmark
        .destination()
        .and_then(|destination| destination.page_index().ok())
        .and_then(|index| usize::try_from(index).ok())
        .map(|index| index + 1);
    OutlineItem { title: bookmark.title().unwrap_or_else(|| "Untitled".to_owned()), target_page, children }
}
