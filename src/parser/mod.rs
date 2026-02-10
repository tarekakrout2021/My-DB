//! SQL Parser using lrlex and lrpar.
//!
//! This module implements a complete SQL parser that can handle DDL (Data Definition Language)
//! and DQL (Data Query Language) statements. The parser is built using the lrlex lexer
//! and lrpar parser generators, which compile grammar files into efficient parsing code.
//!
//! The parsing process:
//! 1. Lexical analysis breaks input into tokens using the `.l` lexer file
//! 2. Syntactic analysis builds an AST using the `.y` grammar file
//! 3. The AST is then passed to semantic analysis for validation and statement creation
//!

pub mod ast;
pub mod expr;

use anyhow::{Result, anyhow};
use ast::AST;
use lrlex::lrlex_mod;
use lrpar::lrpar_mod;

// Include the generated lexer and parser modules
lrlex_mod!("parser/sql.l");
lrpar_mod!("parser/sql.y");

/// SQL parser that converts text input into an Abstract Syntax Tree (AST).
///
/// # Examples
/// ```no_run
/// use imlab::parser::Parser;
///
/// let parser = Parser::default();
/// let ast = parser.parse("select * from foo;").unwrap();
/// ```
#[derive(Default)]
pub struct Parser {}

impl Parser {
    /// Parses input text into an Abstract Syntax Tree.
    ///
    /// Takes a string input and performs lexical and syntactic analysis to produce
    /// an AST that can be used for semantic analysis and statement execution.
    pub fn parse(&self, input: &str) -> Result<AST> {
        let lexerdef = sql_l::lexerdef();

        // create the lexer
        let lexer = lexerdef.lexer(input);
        // pass the lexer to the parser and lex and parse the input
        let (res, errs) = sql_y::parse(&lexer);

        if !errs.is_empty() {
            let mut msg = String::new();
            for e in errs {
                msg.push_str(&e.pp(&lexer, &sql_y::token_epp));
                msg.push('\n');
            }
            return Err(anyhow!(msg));
        }

        // no errors occurred, this should be fine (famous last words)
        match res {
            Some(Ok(ast)) => Ok(ast),
            Some(Err(e)) => Err(anyhow!(e.to_string())),
            None => Err(anyhow!("no parser output")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::Statement;

    #[test]
    fn parse_create_stmt() {
        let p = Parser::default();
        p.parse("CREATE TABLE foo ();").unwrap();
        p.parse("create table foo ();").unwrap();
        p.parse("create table foo (); create table bar ();")
            .unwrap();
        p.parse("create table foo ( bar integer not null );")
            .unwrap();
        p.parse("create table foo ( bar timestamp not null );")
            .unwrap();
        p.parse("create table foo ( bar numeric(4, 2) not null );")
            .unwrap();
        p.parse("create table foo ( bar char(20) not null );")
            .unwrap();
        p.parse("create table foo ( bar varchar(20) not null );")
            .unwrap();

        p.parse(
            "
        create table foo (
            c1 integer not null,
            c2 timestamp not null,
            c3 numeric(4, 2) not null,
            c4 char(20) not null,
            c5 varchar(20) not null
        );
        ",
        )
        .unwrap();

        p.parse(
            "
        create table foo (
            f1 integer not null,
            f2 timestamp not null,
            f3 numeric(4, 2) not null,
            f4 char(20) not null,
            f5 varchar(20) not null
        );

        create table bar (
            b1 integer not null,
            b2 timestamp not null,
            b3 numeric(4, 2) not null,
            b4 char(20) not null,
            b5 varchar(20) not null
        );
        ",
        )
        .unwrap();

        p.parse(
            "
        create table foo (
            c1 integer not null,
            c2 timestamp not null,
            c3 numeric(4, 2) not null,
            c4 char(20) not null,
            c5 varchar(20) not null,
            primary key (c1)
        );
        ",
        )
        .unwrap();

        p.parse(
            "
        create table foo (
            c1 integer not null,
            c2 timestamp not null,
            c3 numeric(4, 2) not null,
            c4 char(20) not null,
            c5 varchar(20) not null,
            primary key (c1, c2)
        );
        ",
        )
        .unwrap();
        let ast = p
            .parse(
                "
        create table warehouse (
            w_id integer not null,
            w_name varchar(10) not null,
            w_street_1 varchar(20) not null,
            w_street_2 varchar(20) not null,
            w_city varchar(20) not null,
            w_state char(2) not null,
            w_zip char(9) not null,
            w_tax numeric(4, 4) not null,
            w_ytd numeric(12, 2) not null,
            primary key (w_id)
        );

        create table district (
            d_id integer not null,
            d_w_id integer not null,
            d_name varchar(10) not null,
            d_street_1 varchar(20) not null,
            d_street_2 varchar(20) not null,
            d_city varchar(20) not null,
            d_state char(2) not null,
            d_zip char(9) not null,
            d_tax numeric(4, 4) not null,
            d_ytd numeric(12, 2) not null,
            d_next_o_id integer not null,
            primary key (d_w_id, d_id)
        );

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

        create table history (
            h_c_id integer not null,
            h_c_d_id integer not null,
            h_c_w_id integer not null,
            h_d_id integer not null,
            h_w_id integer not null,
            h_date timestamp not null,
            h_amount numeric(6, 2) not null,
            h_data varchar(24) not null
        );

        create table neworder (
            no_o_id integer not null,
            no_d_id integer not null,
            no_w_id integer not null,
            primary key (no_w_id, no_d_id, no_o_id)
        );

        create table \"order\" (
            o_id integer not null,
            o_d_id integer not null,
            o_w_id integer not null,
            o_c_id integer not null,
            o_entry_d timestamp not null,
            o_carrier_id integer not null,
            o_ol_cnt numeric(2, 0) not null,
            o_all_local numeric(1, 0) not null,
            primary key (o_w_id, o_d_id, o_id)
        );

        create table orderline (
            ol_o_id integer not null,
            ol_d_id integer not null,
            ol_w_id integer not null,
            ol_number integer not null,
            ol_i_id integer not null,
            ol_supply_w_id integer not null,
            ol_delivery_d timestamp not null,
            ol_quantity numeric(2, 0) not null,
            ol_amount numeric(6, 2) not null,
            ol_dist_info char(24) not null,
            primary key (ol_w_id, ol_d_id, ol_o_id, ol_number)
        );

        create table item (
            i_id integer not null,
            i_im_id integer not null,
            i_name varchar(24) not null,
            i_price numeric(5,2) not null,
            i_data varchar(50) not null,
            primary key (i_id)
        );

        create table stock (
            s_i_id integer not null,
            s_w_id integer not null,
            s_quantity numeric(4, 0) not null,
            s_dist_01 char(24) not null,
            s_dist_02 char(24) not null,
            s_dist_03 char(24) not null,
            s_dist_04 char(24) not null,
            s_dist_05 char(24) not null,
            s_dist_06 char(24) not null,
            s_dist_07 char(24) not null,
            s_dist_08 char(24) not null,
            s_dist_09 char(24) not null,
            s_dist_10 char(24) not null,
            s_ytd numeric(8,0) not null,
            s_order_cnt numeric(4, 0) not null,
            s_remote_cnt numeric(4, 0) not null,
            s_data varchar(50) not null,
            primary key (s_w_id, s_i_id)
        );

        ",
            )
            .unwrap();

        assert_eq!(ast.statements.len(), 9);
        let col_names: Vec<String> = ast
            .statements
            .into_iter()
            .map(|e| {
                if let Statement::CreateTable(x) = e {
                    x.table.id
                } else {
                    panic!("expected A")
                }
            })
            .collect();

        let expected = vec![
            "warehouse",
            "district",
            "customer",
            "history",
            "neworder",
            "order",
            "orderline",
            "item",
            "stock",
        ];

        for expected_name in expected {
            assert!(col_names.contains(&expected_name.to_string()));
        }
    }

    #[test]
    fn parse_copy_stmt() {
        let p = Parser::default();
        p.parse("COPY students FROM 'students.csv';").unwrap();
        p.parse("COPY district FROM 'districts.csv' DELIMITER '|';")
            .unwrap();
    }

    #[test]
    fn parse_select_stmt() {
        let p = Parser::default();
        let ast = p.parse("select * from students;").unwrap();
        let ast = p.parse("select matrnr from students;").unwrap();
        let ast = p.parse("select matrnr, name from students;").unwrap();
        let ast = p.parse("select * from students s;").unwrap();
        let ast = p.parse("select s.matrnr from students s;").unwrap();
        let ast = p.parse("select s.matrnr, s.name from students s;").unwrap();
        let ast = p
            .parse("select * from students, attend, lectures;")
            .unwrap();
        let ast = p
            .parse("select * from students s, attend a, lectures l;")
            .unwrap();
        let ast = p
            .parse("select * from students where name = 'foo';")
            .unwrap();
        let ast = p
            .parse("select * from students s where s.name = 'foo';")
            .unwrap();
        let ast = p
            .parse("select * from students s where s.matrnr = 1234;")
            .unwrap();
        let ast = p
            .parse("select * from students s where s.name = 'foo' and s.matrnr = 1234;")
            .unwrap();
    }
}
