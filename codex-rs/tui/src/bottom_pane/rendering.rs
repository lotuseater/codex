//! Rendering / layout for the bottom pane.
//!
//! This module owns how the `BottomPane` turns its current state (active view, status
//! indicator, footers, composer) into a [`Renderable`] tree, plus the small composer
//! wrapper used to reserve horizontal space on the right edge. The controller logic in
//! `mod.rs` calls these methods; the painting details live here.
use super::*;

impl BottomPane {
    fn as_renderable(&'_ self) -> RenderableItem<'_> {
        self.as_renderable_with_composer_right_reserve(/*composer_right_reserve*/ 0)
    }

    fn as_renderable_with_composer_right_reserve(
        &'_ self,
        composer_right_reserve: u16,
    ) -> RenderableItem<'_> {
        if let Some(view) = self.active_view() {
            RenderableItem::Borrowed(view)
        } else {
            let mut flex = FlexRenderable::new();
            if let Some(status) = &self.status {
                flex.push(/*flex*/ 0, RenderableItem::Borrowed(status));
            }
            // Avoid double-surfacing the same summary and avoid adding an extra
            // row while the status line is already visible.
            if self.status.is_none() && !self.unified_exec_footer.is_empty() {
                flex.push(
                    /*flex*/ 0,
                    RenderableItem::Borrowed(&self.unified_exec_footer),
                );
            }
            let has_pending_thread_approvals = !self.pending_thread_approvals.is_empty();
            let has_pending_input = !self.pending_input_preview.queued_messages.is_empty()
                || !self.pending_input_preview.pending_steers.is_empty()
                || !self.pending_input_preview.rejected_steers.is_empty();
            let has_status_or_footer =
                self.status.is_some() || !self.unified_exec_footer.is_empty();
            let has_inline_previews = has_pending_thread_approvals || has_pending_input;
            if has_inline_previews && has_status_or_footer {
                flex.push(/*flex*/ 0, RenderableItem::Owned("".into()));
            }
            flex.push(
                /*flex*/ 1,
                RenderableItem::Borrowed(&self.pending_thread_approvals),
            );
            if has_pending_thread_approvals && has_pending_input {
                flex.push(/*flex*/ 0, RenderableItem::Owned("".into()));
            }
            flex.push(
                /*flex*/ 1,
                RenderableItem::Borrowed(&self.pending_input_preview),
            );
            if !has_inline_previews && has_status_or_footer {
                flex.push(/*flex*/ 0, RenderableItem::Owned("".into()));
            }
            let mut flex2 = FlexRenderable::new();
            flex2.push(/*flex*/ 1, RenderableItem::Owned(flex.into()));
            let composer: RenderableItem<'_> = if composer_right_reserve == 0 {
                RenderableItem::Borrowed(&self.composer)
            } else {
                RenderableItem::Owned(Box::new(ChatComposerRightReserveRenderable {
                    composer: &self.composer,
                    right_reserve: composer_right_reserve,
                }))
            };
            flex2.push(/*flex*/ 0, composer);
            RenderableItem::Owned(Box::new(flex2))
        }
    }

    pub(crate) fn render_with_composer_right_reserve(
        &self,
        area: Rect,
        buf: &mut Buffer,
        composer_right_reserve: u16,
    ) {
        self.as_renderable_with_composer_right_reserve(composer_right_reserve)
            .render(area, buf);
    }

    pub(crate) fn desired_height_with_composer_right_reserve(
        &self,
        width: u16,
        composer_right_reserve: u16,
    ) -> u16 {
        self.as_renderable_with_composer_right_reserve(composer_right_reserve)
            .desired_height(width)
    }

    pub(crate) fn cursor_pos_with_composer_right_reserve(
        &self,
        area: Rect,
        composer_right_reserve: u16,
    ) -> Option<(u16, u16)> {
        self.as_renderable_with_composer_right_reserve(composer_right_reserve)
            .cursor_pos(area)
    }

    pub(crate) fn cursor_style_with_composer_right_reserve(
        &self,
        area: Rect,
        composer_right_reserve: u16,
    ) -> crossterm::cursor::SetCursorStyle {
        self.as_renderable_with_composer_right_reserve(composer_right_reserve)
            .cursor_style(area)
    }
}

struct ChatComposerRightReserveRenderable<'a> {
    composer: &'a chat_composer::ChatComposer,
    right_reserve: u16,
}

impl Renderable for ChatComposerRightReserveRenderable<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.composer.render_with_mask_and_textarea_right_reserve(
            area,
            buf,
            /*mask_char*/ None,
            self.right_reserve,
        );
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.composer
            .desired_height_with_textarea_right_reserve(width, self.right_reserve)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.composer
            .cursor_pos_with_textarea_right_reserve(area, self.right_reserve)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.composer.cursor_style(area)
    }
}

impl Renderable for BottomPane {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.as_renderable().render(area, buf);
    }
    fn desired_height(&self, width: u16) -> u16 {
        self.as_renderable().desired_height(width)
    }
    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.as_renderable().cursor_pos(area)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.as_renderable().cursor_style(area)
    }
}
