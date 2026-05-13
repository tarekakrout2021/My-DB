use super::row::{
    Key, Row, RowView, ValueRef, decode_value_ref, drop_text_at, encode_field_fixed,
    encode_value_fixed,
};
use super::{Column, Value};
use crate::parser::ast::Table as ASTTable;
use crate::types::Tag;
use std::error::Error;

type FastMap<K, V> = hashbrown::HashMap<K, V, ahash::RandomState>;

/// A row-store table with a compact, contiguous physical layout.
#[derive(Debug)]
pub struct Table {
    pub columns: Vec<Column>,

    /// Row bytes laid out as: row0 || row1 || ...
    pub raw_data: Vec<u8>,
    pub row_size: usize,
    pub num_rows: usize,

    /// Column offsets within a row (packed, no alignment padding).
    pub col_offsets: Vec<usize>,

    /// Primary-key index: key-bytes -> row_id
    pub key_index: FastMap<Vec<u8>, usize>,
    pub key_positions: Vec<usize>,
}

impl Table {
    pub fn new(asttable: &ASTTable) -> Self {
        let mut cols: Vec<Column> = Vec::new();
        let mut key_pos: Vec<usize> = Vec::new();

        for ast_col in &asttable.columns {
            let db_col = Column {
                id: ast_col.id.clone(),
                r#type: ast_col.r#type,
                primary_key: false,
            };
            cols.push(db_col);
        }

        for pk in &asttable.primary_key {
            for (i, col) in cols.iter_mut().enumerate() {
                if col.id == *pk {
                    col.primary_key = true;
                    key_pos.push(i);
                }
            }
        }

        let mut offsets = Vec::with_capacity(cols.len());
        let mut off = 0usize;
        for c in &cols {
            offsets.push(off);
            off += c.size();
        }

        Self {
            row_size: off,
            columns: cols,
            raw_data: Vec::new(),
            num_rows: 0,
            col_offsets: offsets,
            key_index: hashbrown::HashMap::with_hasher(ahash::RandomState::new()),
            key_positions: key_pos,
        }
    }

    #[inline]
    pub fn row_ptr(&self, row_id: usize) -> Option<&[u8]> {
        let start = row_id.checked_mul(self.row_size)?;
        let end = start.checked_add(self.row_size)?;
        self.raw_data.get(start..end)
    }

    #[inline]
    fn row_ptr_mut(&mut self, row_id: usize) -> Option<&mut [u8]> {
        let start = row_id.checked_mul(self.row_size)?;
        let end = start.checked_add(self.row_size)?;
        self.raw_data.get_mut(start..end)
    }

    pub fn insert(&mut self, row: Row) -> Result<(), Box<dyn Error>> {
        if row.len() != self.columns.len() {
            return Err(format!(
                "Column count mismatch: expected {}, got {}",
                self.columns.len(),
                row.len()
            )
            .into());
        }

        let mut row_bytes = Vec::with_capacity(self.row_size);
        for (i, col) in self.columns.iter().enumerate() {
            encode_value_fixed(&mut row_bytes, col.r#type, &row[i])?;
        }
        debug_assert_eq!(row_bytes.len(), self.row_size);

        let row_id = self.num_rows;
        // Build key bytes from the serialized row bytes.
        let key_bytes = self.key_bytes_from_row_bytes(&row_bytes);
        self.key_index.insert(key_bytes, row_id);

        self.raw_data.extend_from_slice(&row_bytes);
        self.num_rows += 1;
        Ok(())
    }

    pub fn insert_fields(&mut self, fields: &[&str]) -> Result<(), Box<dyn Error>> {
        if fields.len() < self.columns.len() {
            return Err(format!(
                "Column count mismatch: expected {}, got {}",
                self.columns.len(),
                fields.len()
            )
            .into());
        }

        let row_id = self.num_rows;

        let start = self.raw_data.len();
        self.raw_data.reserve(self.row_size);

        for (i, col) in self.columns.iter().enumerate() {
            encode_field_fixed(&mut self.raw_data, col.r#type, fields[i])?;
        }
        let end = self.raw_data.len();
        let row_bytes = &self.raw_data[start..end];
        debug_assert_eq!(row_bytes.len(), self.row_size);

        let key_bytes = self.key_bytes_from_row_bytes(row_bytes);
        self.key_index.insert(key_bytes, row_id);
        self.num_rows += 1;
        Ok(())
    }

    pub fn get_row_id(&self, key: &Key) -> Option<usize> {
        let key_bytes = self.key_bytes_from_key_values(key)?;
        self.key_index.get(&key_bytes).cloned()
    }

    /// Zero-copy access to a row by key.
    pub fn get_view(&self, key: &Key) -> Option<RowView<'_>> {
        let key_bytes = self.key_bytes_from_key_values(key)?;
        let row_id = *self.key_index.get(&key_bytes)?;
        Some(RowView {
            table: self,
            row_id,
        })
    }

    /// Overwrite a single field in a row by key
    pub fn put_field(
        &mut self,
        key: &Key,
        col_idx: usize,
        value: &Value,
    ) -> Result<(), Box<dyn Error>> {
        if col_idx >= self.columns.len() {
            return Err("Column index out of bounds".into());
        }

        // Locate row_id from key bytes
        let key_bytes = self
            .key_bytes_from_key_values(key)
            .ok_or_else(|| "Invalid key".to_string())?;
        let row_id = *self
            .key_index
            .get(&key_bytes)
            .ok_or_else(|| "Key not found".to_string())?;

        let col = &self.columns[col_idx];
        let off = self.col_offsets[col_idx];
        let sz = col.size();

        // if the col is text, we potentially need to free
        match col.r#type.r#type() {
            Tag::Text | Tag::Char | Tag::VarChar => {
                let row_ro = self.row_ptr(row_id).ok_or("Row out of bounds")?;
                drop_text_at(row_ro, off);
            }
            _ => {}
        }

        // Encode only this field to a temporary buffer.
        let mut tmp = Vec::with_capacity(sz);
        encode_value_fixed(&mut tmp, col.r#type, value)?;
        debug_assert_eq!(tmp.len(), sz);

        // Overwrite field bytes in-place.
        let row_buf = self.row_ptr_mut(row_id).ok_or("Row out of bounds")?;
        row_buf[off..off + sz].copy_from_slice(&tmp);

        // If PK could change, recompute and update the key index.
        if self.key_positions.contains(&col_idx) {
            let row_ro = self.row_ptr(row_id).ok_or("Row out of bounds")?;
            let new_key_bytes = self.key_bytes_from_row_bytes(row_ro);
            if new_key_bytes != key_bytes {
                self.key_index.remove(&key_bytes);
                self.key_index.insert(new_key_bytes, row_id);
            }
        }

        Ok(())
    }

    pub fn get_col_index(&self, col_name: &str) -> Option<usize> {
        self.columns.iter().position(|col| col.id == col_name)
    }

    fn key_bytes_from_row_bytes(&self, row_bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();

        for &col_i in &self.key_positions {
            let off = self.col_offsets[col_i];
            let col = &self.columns[col_i];

            match col.r#type.r#type() {
                Tag::Text | Tag::Char | Tag::VarChar => {
                    let tv = text_view_at(row_bytes, off);
                    let bytes = tv.as_str().as_bytes(); // actual text content
                    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    out.extend_from_slice(bytes);
                }
                _ => {
                    let sz = col.size();
                    out.extend_from_slice(&row_bytes[off..off + sz]);
                }
            }
        }

        out
    }

    fn key_bytes_from_key_values(&self, key: &Key) -> Option<Vec<u8>> {
        if key.len() != self.key_positions.len() {
            return None;
        }

        let mut out = Vec::new();

        for (k_i, &col_i) in self.key_positions.iter().enumerate() {
            let col_type = self.columns[col_i].r#type;

            match col_type.r#type() {
                Tag::Text | Tag::Char | Tag::VarChar => {
                    let Value::Text(t) = &key[k_i] else {
                        return None;
                    };
                    let bytes = t.as_slice();
                    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    out.extend_from_slice(bytes);
                }
                _ => {
                    encode_value_fixed(&mut out, col_type, &key[k_i]).ok()?;
                }
            }
        }

        Some(out)
    }

    pub fn load_from_file(&mut self, path: &str, delimiter: &str) -> Result<(), Box<dyn Error>> {
        let file = std::fs::File::open(path)?;

        let delim_byte = if delimiter == "\\t" || delimiter == "\t" {
            b'\t'
        } else {
            delimiter.chars().next().ok_or("Empty delimiter")? as u8
        };

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(delim_byte)
            .double_quote(true)
            .escape(Some(b'\\'))
            .from_reader(file);

        for rec in rdr.records() {
            let rec = rec?;
            let mut fields: Vec<&str> = rec.iter().collect();
            if fields.len() > self.columns.len() {
                fields.truncate(self.columns.len()); // some lines might end extra delimiters in imdb, ignore extra fields
            }
            self.insert_fields(&fields)?;
        }

        Ok(())
    }

    pub fn empty_like(&self) -> Self {
        Self {
            columns: self.columns.clone(),
            raw_data: Vec::new(),
            row_size: self.row_size,
            num_rows: 0,
            col_offsets: self.col_offsets.clone(),
            key_index: hashbrown::HashMap::with_hasher(ahash::RandomState::new()),
            key_positions: self.key_positions.clone(),
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        if self.num_rows == 0 {
            return;
        }
        for row_id in 0..self.num_rows {
            if let Some(row) = self.row_ptr(row_id) {
                for (i, col) in self.columns.iter().enumerate() {
                    match col.r#type.r#type() {
                        Tag::Text | Tag::Char | Tag::VarChar => {
                            let off = self.col_offsets[i];
                            drop_text_at(row, off);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

pub(crate) fn text_view_at(row: &[u8], off: usize) -> crate::types::text::TextView {
    let bytes = &row[off..off + 16];
    let len = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let mut data = [0u8; 12];
    data.copy_from_slice(&bytes[4..16]);
    crate::types::text::TextView { len, data }
}

impl Table {
    /// Debug-print a row by row_id, decoding each column.
    pub fn debug_row(&self, row_id: usize) {
        let row = match self.row_ptr(row_id) {
            Some(r) => r,
            None => {
                eprintln!("Row {} out of bounds", row_id);
                return;
            }
        };

        println!("Row {}:", row_id);

        for (i, col) in self.columns.iter().enumerate() {
            let off = self.col_offsets[i];

            print!(" {} : ", col.id);

            let v = decode_value_ref(row, off, col.r#type);
            match v {
                ValueRef::Integer(i) => {
                    print!("Integer({})", i);
                }
                ValueRef::Numeric(n) => {
                    print!("Numeric({})", n);
                }
                ValueRef::Bool(b) => {
                    print!("Bool({})", b);
                }
                ValueRef::Timestamp(t) => {
                    print!("Timestamp({})", t);
                }
                ValueRef::Text(tv) => {
                    print!("Text(\"{}\")", tv.as_str());
                }
            }
        }
        println!()
    }

    pub fn field_bytes(&self, row_id: usize, col_idx: usize) -> Option<&[u8]> {
        let row = self.row_ptr(row_id)?;
        let off = self.col_offsets[col_idx];
        let sz = self.columns[col_idx].size();
        Some(&row[off..off + sz])
    }
}
