use super::{Table, Value};
use crate::types::prelude::Text;
use crate::types::{Tag, Type};
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};

pub type Row = Vec<Value>;
pub type Key = Vec<Value>;

#[derive(Copy, Clone, Debug)]
pub struct RowView<'a> {
    pub table: &'a Table,
    pub row_id: usize,
}

impl<'a> RowView<'a> {
    pub fn get(&self, col_idx: usize) -> Option<ValueRef> {
        let row = self.table.row_ptr(self.row_id)?;
        if col_idx >= self.table.columns.len() {
            return None;
        }
        let col = &self.table.columns[col_idx];
        let off = self.table.col_offsets[col_idx];
        Some(decode_value_ref(row, off, col.r#type))
    }

    pub fn get_bytes(&self, col_idx: usize) -> Option<&'a [u8]> {
        let row = self.table.row_ptr(self.row_id)?;
        let col = self.table.columns.get(col_idx)?;
        let off = self.table.col_offsets[col_idx];
        let sz = col.size();
        row.get(off..off + sz)
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq)]
pub enum ValueRef {
    Bool(bool),
    Integer(i32),
    Numeric(u64),
    Timestamp(i64),
    Text(crate::types::text::TextView),
}

impl ValueRef {
    pub fn value_ref_to_bytes(v: ValueRef) -> Vec<u8> {
        match v {
            ValueRef::Bool(b) => vec![b as u8],
            ValueRef::Integer(i) => i.to_le_bytes().to_vec(),
            ValueRef::Numeric(n) => n.to_le_bytes().to_vec(),
            ValueRef::Timestamp(t) => t.to_le_bytes().to_vec(),
            ValueRef::Text(tv) => tv.as_str().as_bytes().to_vec(),
        }
    }
}

impl PartialEq for ValueRef {
    fn eq(&self, other: &Self) -> bool {
        use ValueRef::*;
        match (self, other) {
            (Bool(a), Bool(b)) => a == b,
            (Integer(a), Integer(b)) => a == b,
            (Numeric(a), Numeric(b)) => a == b,
            (Timestamp(a), Timestamp(b)) => a == b,
            (Text(a), Text(b)) => a.as_str() == b.as_str(),
            _ => false,
        }
    }
}

impl Hash for ValueRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            ValueRef::Bool(b) => b.hash(state),
            ValueRef::Integer(i) => i.hash(state),
            ValueRef::Numeric(n) => n.hash(state),
            ValueRef::Timestamp(t) => t.hash(state),
            ValueRef::Text(tv) => tv.as_str().hash(state),
        }
    }
}

impl fmt::Display for ValueRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValueRef::Bool(b) => write!(f, "{b}"),
            ValueRef::Integer(i) => write!(f, "{i}"),
            ValueRef::Numeric(n) => write!(f, "{n}"),
            ValueRef::Timestamp(t) => write!(f, "{t}"),
            ValueRef::Text(tv) => write!(f, "{}", tv.as_str()),
        }
    }
}

pub fn encode_field_fixed(
    buf: &mut Vec<u8>,
    col_type: Type,
    field: &str,
) -> Result<(), Box<dyn Error>> {
    match col_type.r#type() {
        Tag::Bool => {
            let v = crate::types::bool::Bool::input(field, col_type)?;
            buf.push(v.value as u8);
        }
        Tag::Integer => {
            let v = crate::types::integer::Integer::input(field, col_type)?;
            buf.extend_from_slice(&v.value.to_le_bytes());
        }
        Tag::Numeric => {
            let v = crate::types::numeric::Numeric::input(field, col_type)?;
            buf.extend_from_slice(&v.value.to_le_bytes());
        }
        Tag::Timestamp => {
            let v = crate::types::timestamp::Timestamp::input(field, col_type)?;
            buf.extend_from_slice(&v.value.to_le_bytes());
        }
        Tag::Text | Tag::Char | Tag::VarChar => {
            let t = Text::input(field, col_type)?;
            push_text_struct(buf, t);
        }
    }
    Ok(())
}

pub fn encode_value_fixed(
    buf: &mut Vec<u8>,
    col_type: Type,
    v: &Value,
) -> Result<(), Box<dyn Error>> {
    match (col_type.r#type(), v) {
        (Tag::Bool, Value::Bool(b)) => buf.push(b.value as u8),
        (Tag::Integer, Value::Integer(i)) => buf.extend_from_slice(&i.value.to_le_bytes()),
        (Tag::Numeric, Value::Numeric(n)) => buf.extend_from_slice(&n.value.to_le_bytes()),
        (Tag::Timestamp, Value::Timestamp(ts)) => buf.extend_from_slice(&ts.value.to_le_bytes()),
        (Tag::Text | Tag::Char | Tag::VarChar, Value::Text(t)) => {
            // Store a cloned Text, and forget the clone so the table owns its heap.
            push_text_struct(buf, t.clone());
        }
        _ => {
            return Err(format!(
                "Type mismatch for column: expected {:?}, got {:?}",
                col_type.r#type(),
                v
            )
            .into());
        }
    }
    Ok(())
}

pub fn push_text_struct(buf: &mut Vec<u8>, t: Text) {
    let sz = size_of::<Text>();
    let ptr = &t as *const Text as *const u8;
    unsafe { buf.extend_from_slice(std::slice::from_raw_parts(ptr, sz)) };
    // Prevent drop: table takes ownership of the heap allocation
    std::mem::forget(t);
}

pub fn drop_text_at(row: &[u8], off: usize) {
    // Read 16 bytes as a Text and drop it to free heap allocations.
    let sz = size_of::<Text>();
    let bytes = &row[off..off + sz];
    let mut data = [0u8; 12];
    data.copy_from_slice(&bytes[4..16]);
    let len = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let t = Text { len, data };
    drop(t);
}

pub fn decode_value_fixed(row: &[u8], off: usize, col_type: Type) -> Value {
    match col_type.r#type() {
        Tag::Bool => {
            let b = row[off] != 0;
            crate::types::bool::Bool::new(b).into()
        }
        Tag::Integer => {
            let v = i32::from_le_bytes(row[off..off + 4].try_into().unwrap());
            crate::types::integer::Integer::new(v).into()
        }
        Tag::Numeric => {
            let v = u64::from_le_bytes(row[off..off + 8].try_into().unwrap());
            crate::types::numeric::Numeric::new(v).into()
        }
        Tag::Timestamp => {
            let v = i64::from_le_bytes(row[off..off + 8].try_into().unwrap());
            crate::types::timestamp::Timestamp::new(v).into()
        }
        Tag::Text | Tag::Char | Tag::VarChar => {
            // Borrow and re-allocate into an owned Text for the external API.
            let tv = crate::db::table::text_view_at(row, off);
            let s = tv.as_str();
            Text::input(s, col_type).unwrap().into()
        }
    }
}

pub fn decode_value_ref(row: &[u8], off: usize, col_type: Type) -> ValueRef {
    match col_type.r#type() {
        Tag::Bool => ValueRef::Bool(row[off] != 0),
        Tag::Integer => {
            ValueRef::Integer(i32::from_le_bytes(row[off..off + 4].try_into().unwrap()))
        }
        Tag::Numeric => {
            ValueRef::Numeric(u64::from_le_bytes(row[off..off + 8].try_into().unwrap()))
        }
        Tag::Timestamp => {
            ValueRef::Timestamp(i64::from_le_bytes(row[off..off + 8].try_into().unwrap()))
        }
        Tag::Text | Tag::Char | Tag::VarChar => {
            ValueRef::Text(crate::db::table::text_view_at(row, off))
        }
    }
}
