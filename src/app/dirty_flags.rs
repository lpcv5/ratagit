#[derive(Debug, Default)]
pub struct DirtyFlags {
    pub any: bool,
}

impl DirtyFlags {
    pub fn mark(&mut self) {
        self.any = true;
    }

    pub fn clear(&mut self) {
        self.any = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.any
    }
}
