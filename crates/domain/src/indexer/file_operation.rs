/// Represents the type of operation needed for a file during indexing.
#[derive(Clone)]
pub enum FileOperationNeeded {
    Add,
    Update,
    None,
}
