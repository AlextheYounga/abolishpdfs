pub mod projection;
mod runs;

use crate::model::DocumentModel;

pub use runs::prepare_page;

pub fn prepare(model: &mut DocumentModel) {
    let fonts = &model.fonts;
    for page in &mut model.pages {
        let runs = prepare_page(page, fonts);
        page.prepared_runs = runs;
    }
}
