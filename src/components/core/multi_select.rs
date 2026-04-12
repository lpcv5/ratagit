use std::collections::HashSet;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionMultiplicity {
    SingleOnly,
    BatchCapable,
}

#[derive(Debug, Clone)]
pub struct MultiSelectState<Key> {
    active: bool,
    anchor_index: Option<usize>,
    selected_keys: HashSet<Key>,
}

impl<Key> Default for MultiSelectState<Key>
where
    Key: Eq + Hash,
{
    fn default() -> Self {
        Self {
            active: false,
            anchor_index: None,
            selected_keys: HashSet::new(),
        }
    }
}

impl<Key> MultiSelectState<Key>
where
    Key: Eq + Hash,
{
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn selected_count(&self) -> usize {
        self.selected_keys.len()
    }

    pub fn contains(&self, key: &Key) -> bool {
        self.active && self.selected_keys.contains(key)
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.anchor_index = None;
        self.selected_keys.clear();
    }
}

pub trait MultiSelectableList {
    type Key: Clone + Eq + Hash;

    fn multi_select_state(&self) -> &MultiSelectState<Self::Key>;
    fn multi_select_state_mut(&mut self) -> &mut MultiSelectState<Self::Key>;

    fn toggle_multi_select(
        &mut self,
        cursor_index: Option<usize>,
        visible_keys: &[Self::Key],
    ) -> bool {
        let state = self.multi_select_state_mut();
        if state.active {
            state.clear();
            return false;
        }

        state.active = true;
        state.anchor_index = cursor_index;
        update_visible_range_selection(state, cursor_index, visible_keys);
        true
    }

    fn exit_multi_select(&mut self) {
        self.multi_select_state_mut().clear();
    }

    fn refresh_multi_range(&mut self, cursor_index: Option<usize>, visible_keys: &[Self::Key]) {
        let state = self.multi_select_state_mut();
        if !state.active {
            return;
        }
        update_visible_range_selection(state, cursor_index, visible_keys);
    }

    fn multi_selected_keys(&self, visible_keys: &[Self::Key]) -> Vec<Self::Key> {
        let state = self.multi_select_state();
        if !state.active {
            return Vec::new();
        }

        visible_keys
            .iter()
            .filter(|key| state.selected_keys.contains(*key))
            .cloned()
            .collect()
    }

    fn multi_anchor_key(&self, visible_keys: &[Self::Key]) -> Option<Self::Key> {
        let state = self.multi_select_state();
        if !state.active {
            return None;
        }
        state
            .anchor_index
            .and_then(|idx| visible_keys.get(idx))
            .cloned()
    }

    fn is_multi_active(&self) -> bool {
        self.multi_select_state().is_active()
    }

    fn multi_selected_count(&self) -> usize {
        self.multi_select_state().selected_count()
    }

    fn is_multi_selected_key(&self, key: &Self::Key) -> bool {
        self.multi_select_state().contains(key)
    }
}

fn update_visible_range_selection<Key>(
    state: &mut MultiSelectState<Key>,
    cursor_index: Option<usize>,
    visible_keys: &[Key],
) where
    Key: Clone + Eq + Hash,
{
    state.selected_keys.clear();

    if visible_keys.is_empty() {
        state.anchor_index = None;
        return;
    }

    let Some(cursor) = cursor_index.and_then(|idx| (idx < visible_keys.len()).then_some(idx))
    else {
        return;
    };

    let anchor = state
        .anchor_index
        .and_then(|idx| (idx < visible_keys.len()).then_some(idx))
        .unwrap_or(cursor);
    state.anchor_index = Some(anchor);

    let start = anchor.min(cursor);
    let end = anchor.max(cursor);
    for key in &visible_keys[start..=end] {
        state.selected_keys.insert(key.clone());
    }
}
