use uuid::Uuid;

/// The Document struct is a basic struct that represents a Document with it's title and content, it's unique ID, and the latest modification date.
/// It is used to represent a document in the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The title
    pub path: String,
    /// The content
    pub content: Option<String>,
    /// The uuid
    pub uuid: String,
    /// The creation date
    pub modification_date: i64,
}

impl Document {
    pub fn new(path: String, content: Option<String>, modification_date: i64) -> Self {
        return Self {
            content: content,
            modification_date: modification_date,
            path: path,
            uuid: Uuid::new_v4().to_string(),
        };
    }

    pub fn from_parts(
        path: String,
        content: Option<String>,
        uuid: String,
        modification_date: i64,
    ) -> Self {
        return Self {
            path: path,
            content: content,
            uuid: uuid,
            modification_date: modification_date,
        };
    }
}
