use serde::{Deserialize, Serialize};

pub const CREATED: &str = "{{Type}}Created";
pub const RENAMED: &str = "{{Type}}Renamed";
pub const DELETED: &str = "{{Type}}Deleted";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{Type}}Created {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{Type}}Renamed {
    pub name: String,
}
