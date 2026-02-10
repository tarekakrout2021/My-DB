//! COPY statement implementation.
//!
//! This module implements the CopyStatement which handles COPY operations
//! for loading data from files into database tables. Supports CSV/TBL file
//! formats with configurable delimiters.

use crate::db::Database;
use crate::parser::ast::CopyTable;
use crate::statement::Statement;
use std::error::Error;
use std::io::Write;

/// Executable COPY statement for loading data from files.
#[derive(Debug)]
pub struct CopyStatement {
    copy_table: CopyTable,
}

impl CopyStatement {
    pub fn new(copy_table: CopyTable) -> Self {
        Self { copy_table }
    }
}
impl Statement for CopyStatement {
    /// Prepares the COPY statement for execution.
    fn prepare(&mut self, _db: &mut Database) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// Executes the COPY statement.
    fn execute(&mut self, db: &mut Database, out: &mut dyn Write) -> Result<(), Box<dyn Error>> {
        let _ = db.copy_table(&self.copy_table)?;
        writeln!(
            out,
            "Copied '{}' to '{}'.",
            self.copy_table.file, self.copy_table.table
        )?;
        Ok(())
    }
}
