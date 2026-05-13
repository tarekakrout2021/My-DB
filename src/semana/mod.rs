//! Semantic analysis for SQL statements.
//!
//! This module performs semantic analysis on parsed AST nodes to validate
//! their correctness and convert them into executable Statement objects.
//! Semantic analysis includes:
//!
//! - Type checking and validation
//! - Reference resolution (table and column names)
//! - Constraint validation (primary keys)
//! - Scope management and name resolution

pub mod scope;
pub mod semana_errors;

use std::rc::Rc;

use crate::algebra::Op;
use crate::algebra::iu::{IU, IURef};
use crate::algebra::join::Join;
use crate::algebra::print::Print;
use crate::algebra::scan::TableScan;
use crate::algebra::select::Select as SelectOperator;
use crate::db::Database;
use crate::parser::ast::Statement as ASTStmt;
use crate::parser::ast::{ColId, CreateTable as ASTCreateStmt};
use crate::parser::ast::{CopyTable as ASTCopyStmt, TableFactor};
use crate::parser::expr::Expression::BinaryExpr;
use crate::parser::expr::{BinaryExpression, Expression};
use crate::semana::scope::{IUMap, Scope};
use crate::semana::semana_errors::SemanticError;
use crate::semana::semana_errors::SemanticError::{
    ColumnNotNull, FileNotFound, KeyNotInColumns, RepeatedColumnNames, TableExists, TableNotExists,
};
use crate::statement::Statement;
use crate::statement::copy::CopyStatement;
use crate::statement::create::CreateStatement;
use crate::statement::select::SelectStatement;
use std::collections::HashSet;
use std::path::Path;

/// Semantic analyzer that validates AST nodes and produces executable statements.
///
/// The SemanticAnalysis struct holds a reference to the database object
/// to perform validation checks like ensuring table names don't conflict
/// and column references are valid.
pub struct SemanticAnalysis<'a> {
    /// Reference to the database for schema validation
    db: &'a Database,
}

type ASTSelectStmt = crate::parser::ast::Query;
type ASTFrom = Vec<TableFactor>;
type ASTWhere = Vec<BinaryExpression>;
type ASTTarget = Vec<ColId>;
type ASTTableRef = TableFactor;

impl SemanticAnalysis<'_> {
    /// Creates a new semantic analyzer with access to the database.
    pub fn new(db: &'_ Database) -> SemanticAnalysis<'_> {
        SemanticAnalysis { db }
    }

    /// Analyzes an AST statement and converts it to an executable Statement.
    pub fn analyze(&self, ast_stmt: ASTStmt) -> Result<Box<dyn Statement>, SemanticError> {
        // Your AST should consist of a vector of statements, call analyze() for each statement
        match ast_stmt {
            ASTStmt::CreateTable(ct) => Ok(Box::new(self.analyze_create_stmt(&ct)?)),
            ASTStmt::CopyTable(ct) => Ok(Box::new(self.analyze_copy_stmt(&ct)?)),
            ASTStmt::Select(q) => Ok(Box::new(self.analyze_select_stmt(&q)?)),
        }
    }

    /// Analyzes a COPY statement AST node.
    fn analyze_copy_stmt(&self, stmt: &ASTCopyStmt) -> Result<CopyStatement, SemanticError> {
        let Some(_) = self.db.get_table(&stmt.table) else {
            return Err(TableNotExists(stmt.table.clone()));
        };

        // assert file exists
        let path = Path::new(&stmt.file);
        if !path.exists() {
            return Err(FileNotFound(stmt.file.to_string()));
        }

        Ok(CopyStatement::new(stmt.clone()))
    }

    /// Analyzes a CREATE TABLE statement AST node.
    fn analyze_create_stmt(&self, stmt: &ASTCreateStmt) -> Result<CreateStatement, SemanticError> {
        //Table name doesn't already exist
        let db_name = &stmt.table.id;
        if self.db.tables.contains_key(db_name) {
            return Err(TableExists);
        }
        //Column names are unique within the table
        let mut columns_set = HashSet::new();
        for col in stmt.table.columns.iter() {
            columns_set.insert(col.id.clone());
        }

        if stmt.table.columns.len() > columns_set.len() {
            return Err(RepeatedColumnNames);
        }
        // Data types are valid and supported
        for col in stmt.table.columns.iter() {
            if !col.not_null {
                return Err(ColumnNotNull(col.id.clone()));
            }
        }

        //Primary key references valid columns
        for pk in stmt.table.primary_key.iter() {
            if !columns_set.contains(pk) {
                return Err(KeyNotInColumns(pk.clone()));
            }
        }
        Ok(CreateStatement::new(stmt.table.clone()))
    }

    /// Analyzes a SELECT statement AST node.
    fn analyze_select_stmt(&self, stmt: &ASTSelectStmt) -> Result<SelectStatement, SemanticError> {
        let mut scope = Scope::default();
        let op = self.analyze_select(stmt, &mut scope)?;

        let mut print_op = Print::new(op);
        if let Some(targets) = &stmt.targets {
            if targets.is_empty() {
                for (_, map) in &scope.ius {
                    for m in map {
                        print_op.iu_refs_set.insert(m.iu.clone());
                        print_op.iu_refs_vec.push(m.iu.clone());
                    }
                }
            } else {
                for col in targets {
                    let iu_ref = scope.iu(col)?;
                    print_op.iu_refs_set.insert(iu_ref.clone());
                    print_op.iu_refs_vec.push(iu_ref.clone());
                }
            }
            let top = Rc::new(Op::Print(print_op));

            Ok(SelectStatement::new(top))
        } else {
            let top = Rc::new(Op::Print(print_op));
            Ok(SelectStatement::new(top))
        }
    }

    /// Builds relational algebra tree from SELECT statement components.
    fn analyze_select(
        &self,
        stmt: &ASTSelectStmt,
        scope: &mut Scope,
    ) -> Result<Rc<Op>, SemanticError> {
        let refs: &ASTFrom = &stmt.from;
        let r#where = stmt.r#where.clone();
        let target = stmt.targets.clone();

        let from_op = self.analyze_from(refs, scope)?;

        let where_op = self.analyze_where(&r#where, scope, from_op)?;

        let res = self.analyze_target(&target, scope, where_op)?;

        Ok(res)
    }

    /// Analyzes FROM clause and builds table scan operators.
    fn analyze_from(&self, from: &ASTFrom, scope: &mut Scope) -> Result<Rc<Op>, SemanticError> {
        // No joins here
        if from.len() == 1 {
            return Ok(self.analyze_table_reference(&from[0], self.db, scope)?);
        }

        // otherwise join
        let first = &from[0];
        let mut accumulator = self.analyze_table_reference(first, self.db, scope)?;

        for tref in &from[1..] {
            let right = self.analyze_table_reference(tref, self.db, scope)?;
            let joined = Join::new(accumulator, right);
            accumulator = Rc::new(Op::Join(joined));
        }

        Ok(accumulator)
    }

    /// Analyzes WHERE clause and adds selection operators.
    fn analyze_where(
        &self,
        r#where: &ASTWhere,
        scope: &mut Scope,
        op: Rc<Op>,
    ) -> Result<Rc<Op>, SemanticError> {
        let mut prev = op;
        for bin_exp in r#where {
            let analyzed_expr =
                self.analyze_expression(&BinaryExpr(Box::new(bin_exp.clone())), scope)?;
            let analyzed_bin = match analyzed_expr {
                Expression::BinaryExpr(b) => *b,
                _ => unreachable!("analyze_expression(BinaryExpr) must return BinaryExpr"),
            };
            let select_op = SelectOperator::new(prev.clone(), analyzed_bin.clone());
            prev = Rc::new(Op::Select(select_op));
            // scope ?
        }

        Ok(prev)
    }

    /// Analyzes SELECT target list and updates scope.
    fn analyze_target(
        &self,
        target: &Option<ASTTarget>,
        scope: &mut Scope,
        op: Rc<Op>,
    ) -> Result<Rc<Op>, SemanticError> {
        // todo: change the IU scope such that the only remaining IUs are the ones referenced in the target list
        //       (you do not have to change the passed operator ... why are we passing the operator to this function
        //       then? because this is the place where map operators would be put on top of the tree, e.g., if the
        //       target was an expression like a + 2)

        if let Some(v) = target {
            if v.len() == 0 {
                return Ok(op);
            }

            // collect all IURefs referenced by the target
            let mut selected_set: HashSet<IURef> = HashSet::new();
            for col in v {
                let iu_ref = scope.iu(col)?;
                selected_set.insert(iu_ref.clone());
            }
            Ok(op)
        } else {
            Ok(op)
        }
    }

    /// Performs type checking and column resolution on expressions.
    fn analyze_expression(
        &self,
        expr: &Expression,
        scope: &mut Scope,
    ) -> Result<Expression, SemanticError> {
        // todo: check types and replace all ColumnRef (if column can be found in scope) with IURef
        match expr {
            Expression::String(_) => Ok(expr.clone()),

            Expression::ColumnRef(col_id) => {
                let iu = scope.iu(col_id)?; // may throw UnknownTable / UnknownColumn / AmbiguousColumn
                Ok(Expression::IURef(iu))
            }

            Expression::IURef(_) => Ok(expr.clone()),

            Expression::BinaryExpr(b) => {
                let left = self.analyze_expression(&b.l, scope)?;
                let right = self.analyze_expression(&b.r, scope)?;

                // left and right should now be either a IURef or a String
                //TODO: should check types and throw a SemanticError::TypeMismatch

                Ok(BinaryExpr(Box::new(BinaryExpression {
                    l: left,
                    r: right,
                    op: b.op,
                })))
            }
            Expression::Subquery(q) => {
                let mut sub_scope = Scope::default();
                let _sub_op = self.analyze_select(q, &mut sub_scope)?;

                // if q.targets.len() != 1 {
                //     return Err(SemanticError::TypeMismatch(
                //         "subquery returns multiple columns".to_string(),
                //         "scalar".to_string(),
                //     ));
                // }

                Ok(expr.clone())
            }
        }
    }

    /// Analyzes table references and creates scan operators.
    fn analyze_table_reference(
        &self,
        r#ref: &ASTTableRef,
        db: &Database,
        scope: &mut Scope,
    ) -> Result<Rc<Op>, SemanticError> {
        match r#ref {
            TableFactor::Table(tref) => {
                let mut table_scan_op = TableScan::new();

                let table_name = tref.table.clone();
                let alias = tref.alias.clone();

                let table = db
                    .get_table(&table_name)
                    .ok_or_else(|| TableNotExists(table_name.clone()))?;

                table_scan_op.table_name = table_name.clone();

                // Build one IU per column
                let mut iu_vec: Vec<IURef> = Vec::with_capacity(table.columns.len());
                for col in &table.columns {
                    let iu = IURef(Rc::new(IU {
                        name: col.id.clone(),
                        r#type: col.r#type.clone(),
                    }));
                    iu_vec.push(iu);
                }

                // add IUs to the TableScan
                for iu in &iu_vec {
                    table_scan_op.iu_refs_set.insert(iu.clone());
                }

                // add the IUs to the scope
                let iu_map_vec: Vec<IUMap> = table
                    .columns
                    .iter()
                    .zip(iu_vec.into_iter())
                    .map(|(col, iu)| IUMap {
                        alias: col.id.clone(),
                        iu,
                    })
                    .collect();

                let key = if alias.is_empty() {
                    &table_name
                } else {
                    &alias
                };
                scope.add_ius(key, iu_map_vec)?;

                Ok(Rc::new(Op::TableScan(table_scan_op)))
            }
            TableFactor::Derived { query, alias } => {
                let mut sub_scope = Scope::default();
                let sub_op = self.analyze_select(query, &mut sub_scope)?;

                // Collect output IUs from sub_scope
                let mut iu_map_vec: Vec<IUMap> = Vec::new();
                if let Some(targets) = &query.targets {
                    if targets.is_empty() {
                        // SELECT * case: expose all IUs from sub_scope
                        for (_tbl_alias, cols) in &sub_scope.ius {
                            for m in cols {
                                iu_map_vec.push(IUMap {
                                    alias: m.alias.clone(),
                                    iu: m.iu.clone(),
                                });
                            }
                        }
                    } else {
                        for col in targets {
                            let iu = sub_scope.iu(col)?;
                            iu_map_vec.push(IUMap {
                                alias: col.column.clone(),
                                iu,
                            });
                        }
                    }
                    // Add the derived table's IUs to outer scope
                    scope.add_ius(alias, iu_map_vec)?;
                }

                Ok(sub_op)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::parser::ast::Statement::Select;
    use std::error::Error;
    use std::io;

    fn analyze(
        db: &Database,
        p: &Parser,
        input: &str,
    ) -> Result<Box<dyn Statement>, SemanticError> {
        let ast = p.parse(input).unwrap();
        let s = SemanticAnalysis::new(db);
        s.analyze(ast.statements[0].clone())
    }

    fn create_table(db: &mut Database, p: &Parser, create_str: &str) {
        let mut st = analyze(&db, &p, create_str).unwrap();
        st.prepare(db).unwrap();
        let mut stdout = io::stdout();
        st.execute(db, &mut stdout).unwrap();
    }

    #[test]
    fn semana_analyze_create() -> Result<(), Box<dyn Error>> {
        let mut db = Database::default();
        let p = Parser::default();

        // CREATE TABLE
        let mut st = analyze(&db, &p, "CREATE TABLE test_table (col1 INTEGER NOT NULL);")?;
        st.prepare(&mut db)?;
        let mut stdout = io::stdout();
        st.execute(&mut db, &mut stdout)?;

        assert_eq!(db.tables.len(), 1);

        let mut st = analyze(
            &db,
            &p,
            "CREATE TABLE test_table_2 (col1 INTEGER NOT NULL, primary key (col1));",
        )?;
        st.prepare(&mut db)?;
        let mut stdout = io::stdout();
        st.execute(&mut db, &mut stdout)?;

        assert_eq!(db.tables.len(), 2);

        let st = analyze(&db, &p, "CREATE TABLE test_table ();");
        assert_eq!(st.unwrap_err(), TableExists);

        let st = analyze(
            &db,
            &p,
            "CREATE TABLE another_table ( col1 INTEGER NOT NULL, col1 INTEGER NOT NULL );",
        );
        assert_eq!(st.unwrap_err(), RepeatedColumnNames);

        assert_eq!(db.tables.len(), 2);

        let st = analyze(
            &db,
            &p,
            "CREATE TABLE another_table ( col1 INTEGER NOT NULL, col2 INTEGER NOT NULL, primary key (col3) );",
        );
        assert_eq!(st.unwrap_err(), KeyNotInColumns("col3".to_string()));

        Ok(())
    }
    #[test]
    #[ignore] // ignore for CI
    fn semana_analyze_copy() -> Result<(), Box<dyn Error>> {
        let mut db = Database::default();
        let p = Parser::default();

        let st = analyze(&db, &p, "COPY customer FROM 'data/tpcc/customer.tbl';");
        assert_eq!(st.unwrap_err(), TableNotExists("customer".to_string()));

        // create table first
        let create_st = "
            create table customer (
            c_id integer not null,
            c_d_id integer not null,
            c_w_id integer not null,
            c_first varchar(16) not null,
            c_middle char(2) not null,
            c_last varchar(16) not null,
            c_street_1 varchar(20) not null,
            c_street_2 varchar(20) not null,
            c_city varchar(20) not null,
            c_state char(2) not null,
            c_zip char(9) not null,
            c_phone char(16) not null,
            c_since timestamp not null,
            c_credit char(2) not null,
            c_credit_lim numeric(12, 2) not null,
            c_discount numeric(4, 4) not null,
            c_balance numeric(12, 2) not null,
            c_ytd_paymenr numeric(12, 2) not null,
            c_payment_cnt numeric(4, 0) not null,
            c_delivery_cnt numeric(4, 0) not null,
            c_data varchar(500) not null,
            primary key (c_w_id, c_d_id, c_id)
        );
        ";
        let mut st = analyze(&db, &p, create_st)?;
        st.prepare(&mut db)?;
        let mut stdout = io::stdout();
        st.execute(&mut db, &mut stdout)?;

        // COPY TABLE
        let mut st = analyze(&db, &p, "COPY customer FROM 'data/tpcc/customer.tbl';")?;
        st.prepare(&mut db)?;
        let mut stdout = io::stdout();
        st.execute(&mut db, &mut stdout)?;

        assert_eq!(db.tables.len(), 1);
        assert_eq!(db.get_table("customer").unwrap().num_rows, 150000);
        Ok(())
    }

    #[test]
    fn analyze_table_ref_test() -> Result<(), Box<dyn Error>> {
        let mut db = Database::default();
        let p = Parser::default();

        // CREATE TABLE
        create_table(
            &mut db,
            &p,
            "CREATE TABLE students (col1 INTEGER NOT NULL, col2 INTEGER NOT NULL, col3 INTEGER NOT NULL);",
        );
        create_table(
            &mut db,
            &p,
            "CREATE TABLE hoeren (h1 INTEGER NOT NULL, h2 INTEGER NOT NULL, h3 INTEGER NOT NULL);",
        );
        create_table(
            &mut db,
            &p,
            "CREATE TABLE another_test (col1 INTEGER NOT NULL);",
        );

        let s = SemanticAnalysis::new(&db);

        // "select col1, asdfasdf from students s, hoeren a, another_test at where h1 = 2;"
        // -> unknown column error...
        // "select * from students s, hoeren a, table t,  another_test at where h1 = 2;"
        // -> table doesnt exist error...
        //"select col1 from students s, hoeren a, another_test at where col1 = 2;"
        // -> ambiguous column error
        //"select h1 from students s, hoeren a, another_test at where name = 'foo';"
        // -> UnknownColumnName "name"

        // let ast =
        //     p.parse("select h1 from students s, hoeren a, another_test at where h1 = 'asdas';")?;

        let ast = p.parse("select * from students;")?;

        let _ = match ast.statements.first() {
            Some(Select(query)) => query,
            _ => panic!("Expected a SELECT statement"),
        };

        let mut res = s.analyze(ast.statements[0].clone())?;
        println!("{:#?}", res);

        res.prepare(&mut db)?;

        Ok(())
    }
}
