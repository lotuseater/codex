//! Geometric layout and cursor-positioning helpers for [`ChatComposer`].

use super::*;

impl ChatComposer {
    pub(super) fn layout_areas(&self, area: Rect) -> [Rect; 4] {
        self.layout_areas_with_textarea_right_reserve(area, /*textarea_right_reserve*/ 0)
    }

    pub(super) fn layout_areas_with_textarea_right_reserve(
        &self,
        area: Rect,
        textarea_right_reserve: u16,
    ) -> [Rect; 4] {
        let footer_props = self.footer_props();
        let footer_hint_height = self
            .custom_footer_height()
            .unwrap_or_else(|| footer_height(&footer_props));
        let footer_spacing = Self::footer_spacing(footer_hint_height);
        let footer_total_height = footer_hint_height + footer_spacing;
        let popup_constraint = match &self.active_popup {
            ActivePopup::Command(popup) => {
                Constraint::Max(popup.calculate_required_height(area.width))
            }
            ActivePopup::File(popup) => Constraint::Max(popup.calculate_required_height()),
            ActivePopup::Skill(popup) => {
                Constraint::Max(popup.calculate_required_height(area.width))
            }
            ActivePopup::MentionV2(popup) => {
                Constraint::Max(popup.calculate_required_height(area.width))
            }
            ActivePopup::None => Constraint::Max(footer_total_height),
        };
        let [composer_rect, popup_rect] =
            Layout::vertical([Constraint::Min(3), popup_constraint]).areas(area);
        let mut textarea_rect = composer_rect.inset(Insets::tlbr(
            /*top*/ 1,
            LIVE_PREFIX_COLS,
            /*bottom*/ 1,
            /*right*/ 1u16.saturating_add(textarea_right_reserve),
        ));
        let remote_images_height = self
            .remote_images_lines(textarea_rect.width)
            .len()
            .try_into()
            .unwrap_or(u16::MAX)
            .min(textarea_rect.height.saturating_sub(1));
        let remote_images_separator = u16::from(remote_images_height > 0);
        let consumed = remote_images_height.saturating_add(remote_images_separator);
        let remote_images_rect = Rect {
            x: textarea_rect.x,
            y: textarea_rect.y,
            width: textarea_rect.width,
            height: remote_images_height,
        };
        textarea_rect.y = textarea_rect.y.saturating_add(consumed);
        textarea_rect.height = textarea_rect.height.saturating_sub(consumed);
        [composer_rect, remote_images_rect, textarea_rect, popup_rect]
    }

    pub(super) fn footer_spacing(footer_hint_height: u16) -> u16 {
        if footer_hint_height == 0 {
            0
        } else {
            FOOTER_SPACING_HEIGHT
        }
    }

    pub fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.cursor_pos_with_textarea_right_reserve(area, /*textarea_right_reserve*/ 0)
    }

    pub(crate) fn cursor_pos_with_textarea_right_reserve(
        &self,
        area: Rect,
        textarea_right_reserve: u16,
    ) -> Option<(u16, u16)> {
        if !self.input_enabled || self.selected_remote_image_index.is_some() {
            return None;
        }

        if let Some(pos) = self.history_search_cursor_pos(area) {
            return Some(pos);
        }

        let [_, _, textarea_rect, _] =
            self.layout_areas_with_textarea_right_reserve(area, textarea_right_reserve);
        let state = *self.textarea_state.borrow();
        self.textarea.cursor_pos_with_state(textarea_rect, state)
    }
}
