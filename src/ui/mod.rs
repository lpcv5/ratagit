pub mod components;
mod highlight;
pub mod layout;
pub mod panels;
pub mod theme;
mod view;
mod views;
pub mod widgets;

pub use view::View;

/// Keep some breathing room around selected rows in scrollable lists.
pub const LIST_SCROLL_PADDING: usize = 2;
