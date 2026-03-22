#[derive(Debug, Default)]
pub struct DirtyFlags {
    pub left_panels: bool,
    pub diff: bool,
    pub command_log: bool,
    pub shortcut_bar: bool,
    pub overlay: bool,
}

impl DirtyFlags {
    pub fn mark_all(&mut self) {
        self.left_panels = true;
        self.diff = true;
        self.command_log = true;
        self.shortcut_bar = true;
        self.overlay = true;
    }

    pub fn mark_diff(&mut self) {
        self.diff = true;
    }

    pub fn mark_command_log(&mut self) {
        self.command_log = true;
    }

    pub fn mark_overlay(&mut self) {
        self.overlay = true;
    }

    pub fn mark_main_content(&mut self) {
        self.left_panels = true;
        self.diff = true;
    }

    pub fn clear(&mut self) {
        self.left_panels = false;
        self.diff = false;
        self.command_log = false;
        self.shortcut_bar = false;
        self.overlay = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.left_panels || self.diff || self.command_log || self.shortcut_bar || self.overlay
    }
}
