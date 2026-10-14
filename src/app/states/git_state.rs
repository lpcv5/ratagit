/// Repository operation observations. FilesState and DiffState own their feature data.
#[derive(Clone, Default)]
pub struct GitState {
    pub operation: crate::git::operation::OperationMachine,
}
