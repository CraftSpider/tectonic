use crate::c_api::engine::{font_area, font_layout_engine};
use crate::c_api::ext::OTGR_FONT_FLAG;
use tectonic_xetex_layout::engine::LayoutEngine;

#[no_mangle]
pub unsafe extern "C" fn glyph_height(f: libc::c_int, g: libc::c_int) -> f32 {
    if font_area[f as usize] == OTGR_FONT_FLAG as i32 {
        let (height, _) = font_layout_engine[f as usize]
            .cast::<LayoutEngine>()
            .as_mut()
            .unwrap()
            .font_mut()
            .get_glyph_height_depth(g as _);
        height
    } else {
        0.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn glyph_depth(f: libc::c_int, g: libc::c_int) -> f32 {
    if font_area[f as usize] == OTGR_FONT_FLAG as i32 {
        let (_, depth) = font_layout_engine[f as usize]
            .cast::<LayoutEngine>()
            .as_mut()
            .unwrap()
            .font_mut()
            .get_glyph_height_depth(g as _);
        depth
    } else {
        0.0
    }
}
