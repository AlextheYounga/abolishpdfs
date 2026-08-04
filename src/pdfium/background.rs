use std::{collections::HashSet, io::Cursor};

use image::ImageFormat;
use pdfium_render::prelude::*;

use crate::model::RasterBackground;

pub(super) fn render_page_background(
    page: &PdfPage<'_>,
    fallback_paint_orders: &[usize],
) -> Result<RasterBackground, PdfiumError> {
    let fallback_paint_orders = fallback_paint_orders.iter().copied().collect::<HashSet<_>>();
    set_native_text_activity(page, &fallback_paint_orders, false)?;
    let rendered =
        page.render_with_config(&PdfRenderConfig::new().set_target_width(1200)).and_then(|bitmap| encode_png(&bitmap));
    let restore_result = set_native_text_activity(page, &fallback_paint_orders, true);
    restore_result.and(rendered)
}

fn set_native_text_activity(
    page: &PdfPage<'_>,
    fallback_paint_orders: &HashSet<usize>,
    active: bool,
) -> Result<(), PdfiumError> {
    let mut paint_order = 0;
    for mut object in page.objects().iter() {
        set_native_object_activity(&mut object, fallback_paint_orders, &mut paint_order, active)?;
    }
    Ok(())
}

fn set_native_object_activity(
    object: &mut PdfPageObject<'_>,
    fallback_paint_orders: &HashSet<usize>,
    paint_order: &mut usize,
    active: bool,
) -> Result<(), PdfiumError> {
    let current_order = *paint_order;
    *paint_order += 1;
    match object {
        PdfPageObject::Text(_) if !fallback_paint_orders.contains(&current_order) => {
            if active {
                object.set_active()
            } else {
                object.set_inactive()
            }
        }
        PdfPageObject::XObjectForm(form) => {
            for mut child in form.iter() {
                set_native_object_activity(&mut child, fallback_paint_orders, paint_order, active)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn encode_png(bitmap: &PdfBitmap<'_>) -> Result<RasterBackground, PdfiumError> {
    let image = bitmap.as_image()?;
    let mut png = Cursor::new(Vec::new());
    image.write_to(&mut png, ImageFormat::Png).map_err(|_| PdfiumError::ImageError)?;
    Ok(RasterBackground {
        width: u32::try_from(bitmap.width()).unwrap_or_default(),
        height: u32::try_from(bitmap.height()).unwrap_or_default(),
        png: png.into_inner(),
    })
}
