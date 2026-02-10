//! CREATE TABLE statement implementation.

use std::error::Error;
use std::io::Write;

use crate::db::Database;
use crate::parser::ast::Table as ASTTable;
use crate::statement::Statement;

/// Executable CREATE TABLE statement.
#[derive(Debug)]
pub struct CreateStatement {
    table: ASTTable,
}
impl CreateStatement {
    pub fn new(table: ASTTable) -> Self {
        Self { table }
    }
}

impl Statement for CreateStatement {
    /// Prepares the CREATE TABLE statement for execution.
    fn prepare(&mut self, _db: &mut Database) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// Executes the CREATE TABLE statement.
    fn execute(&mut self, db: &mut Database, out: &mut dyn Write) -> Result<(), Box<dyn Error>> {
        let _ = db.create_table(&self.table);
        writeln!(out, "Table '{}' created.", self.table.id)?;
        Ok(())
    }
}
