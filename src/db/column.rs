use crate::types::{Tag, Type};

#[derive(Debug, Clone)]
pub struct Column {
    pub id: String,
    pub r#type: Type,
    pub primary_key: bool,
}

impl Column {
    pub fn size(&self) -> usize {
        match self.r#type.r#type() {
            Tag::Bool => 1,
            Tag::Integer => 4,
            Tag::Numeric => 8,
            Tag::Timestamp => 8,
            Tag::Text | Tag::Char | Tag::VarChar => 16,
        }
    }
}
