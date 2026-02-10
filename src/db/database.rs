use super::{Row, Table, Value};
use crate::parser::ast::{CopyTable, Table as ASTTable};
use crate::types::Tag;
use std::collections::HashMap;
use std::error::Error;

#[derive(Default)]
pub struct Database {
    pub tables: HashMap<String, Table>,
}

impl Database {
    /// Creates a new table in the database.
    pub fn create_table(&mut self, table: &ASTTable) -> Result<(), Box<dyn Error>> {
        let t = Table::new(table);
        self.tables.insert(table.id.clone(), t);
        Ok(())
    }

    /// Returns an immutable reference to the specified table.
    pub fn get_table(&self, table: &str) -> Option<&Table> {
        self.tables.get(table)
    }

    /// Returns a mutable reference to the specified table.
    pub fn get_table_mut(&mut self, table: &str) -> Option<&mut Table> {
        self.tables.get_mut(table)
    }

    pub fn copy_table(&mut self, copy_table: &CopyTable) -> Result<(), Box<dyn Error>> {
        let table = self
            .get_table_mut(&copy_table.table)
            .ok_or("Table not found")?;

        table.load_from_file(&copy_table.file, &copy_table.delimiter)
    }
}

pub fn insert_row_into_table(table: &mut Table, row: &[&str]) -> Result<(), Box<dyn Error>> {
    let mut insert_row: Row = Vec::with_capacity(table.columns.len());
    for (i, field) in row.iter().enumerate() {
        let col_type = table.columns[i].r#type;
        let tag = col_type.r#type();
        let value: Value = match tag {
            Tag::Numeric => crate::types::numeric::Numeric::input(field, col_type)?.into(),
            Tag::Bool => crate::types::bool::Bool::input(field, col_type)?.into(),
            Tag::Integer => crate::types::integer::Integer::input(field, col_type)?.into(),
            Tag::Text | Tag::Char | Tag::VarChar => {
                crate::types::text::Text::input(field, col_type)?.into()
            }
            Tag::Timestamp => crate::types::timestamp::Timestamp::input(field, col_type)?.into(),
        };

        insert_row.push(value);
    }
    table.insert(insert_row)
}

#[cfg(test)]
mod tests {
    use csv::ReaderBuilder;

    #[test]
    fn parse_sample_quoted_field_with_escaped_quote() {
        let line = r#"8153,29082,868,1,"(2010) (USA) (DVD) (Included in \"A Touch of Frost: Seasom 15\" for ITV Global Entertainment)""#;
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b',')
            .double_quote(true)
            .escape(Some(b'\\'))
            .from_reader(line.as_bytes());

        let rec = rdr.records().next().expect("record").expect("ok record");
        assert_eq!(rec.len(), 5);
        assert_eq!(rec.get(0).unwrap(), "8153");
        assert_eq!(rec.get(1).unwrap(), "29082");
        assert_eq!(rec.get(2).unwrap(), "868");
        assert_eq!(rec.get(3).unwrap(), "1");
        assert_eq!(
            rec.get(4).unwrap(),
            "(2010) (USA) (DVD) (Included in \"A Touch of Frost: Seasom 15\" for ITV Global Entertainment)"
        );
    }

    #[test]
    fn parse_field_with_internal_comma() {
        // quoted field contains a comma, it should be read as a single field
        let line = r#"1,"Smith, John",3"#;
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b',')
            .double_quote(true)
            .escape(Some(b'\\'))
            .from_reader(line.as_bytes());

        let rec = rdr.records().next().expect("record").expect("ok record");
        assert_eq!(rec.len(), 3);
        assert_eq!(rec.get(0).unwrap(), "1");
        assert_eq!(rec.get(1).unwrap(), "Smith, John");
        assert_eq!(rec.get(2).unwrap(), "3");
    }
}
