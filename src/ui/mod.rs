pub mod components;
mod highlight;
pub mod layout;
pub mod panels;
pub mod theme;
pub mod traits;
pub mod widgets;

pub use panels::selection::RenderState;

#[cfg(test)]
mod golden_tests;
#[cfg(test)]
mod test_support;

/// Keep some breathing room around selected rows in scrollable lists.
pub const LIST_SCROLL_PADDING: usize = 2;
