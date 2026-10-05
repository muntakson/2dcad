// ViewCube stub — 3D navigation removed for 2D-only laser-cut app.
// Provides empty implementations so the view module compiles.

use iced::{Element, Theme};

pub(in crate::app) const UCS_PICKER_W: f32 = 0.0;

pub(in crate::app) fn viewcube_nav_controls<'a>(
    _tab: &super::super::document::DocumentTab,
    _viewport_size: iced::Size,
) -> Option<Element<'a, super::super::Message>> {
    None
}

pub(in crate::app) fn viewcube_ucs_picker<'a>(
    _tab: &super::super::document::DocumentTab,
) -> Option<Element<'a, super::super::Message>> {
    None
}
