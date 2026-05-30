//! Bash-mode toggling and element reconciliation helpers for [`ChatComposer`].

use super::*;

impl ChatComposer {
    pub(super) fn sync_bash_mode_from_text(&mut self) {
        if !self.is_bash_mode && self.textarea.text().starts_with('!') {
            self.textarea.replace_range(0..1, "");
            self.is_bash_mode = true;
        }
    }

    pub(super) fn reconcile_deleted_elements(&mut self, elements_before: Vec<String>) {
        let elements_after: HashSet<String> =
            self.textarea.element_payloads().into_iter().collect();

        let mut removed_any_image = false;
        for removed in elements_before
            .into_iter()
            .filter(|payload| !elements_after.contains(payload))
        {
            self.pending_pastes.retain(|(ph, _)| ph != &removed);

            if let Some(idx) = self
                .attached_images
                .iter()
                .position(|img| img.placeholder == removed)
            {
                self.attached_images.remove(idx);
                removed_any_image = true;
            }
        }

        if removed_any_image {
            self.relabel_attached_images_and_update_placeholders();
        }
    }
}
