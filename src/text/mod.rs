pub mod projection;
pub mod visibility;

mod runs;
mod structure;

use crate::model::DocumentModel;

pub const SPACING_MIN_PX: f32 = 1e-3;

pub use runs::prepare_page;
pub(crate) use structure::{BlockKind, TextBlock, TextStructure};

pub fn prepare(model: &mut DocumentModel) {
    let fonts = &model.fonts;
    for page in &mut model.pages {
        let runs = prepare_page(page, fonts);
        page.prepared_runs = runs;
    }
}
