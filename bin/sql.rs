//! SQL REPL Binary
//!
//! Implement a SQL Read-Eval-Print Loop (REPL) that provides an
//! interactive interface for executing SQL DDL and DQL statements.

use imlab::db::Database;
use imlab::parser::Parser;
use imlab::parser::ast::Statement as ASTStmt;
use imlab::semana::SemanticAnalysis;
use imlab::semana::semana_errors::SemanticError;
use imlab::statement::Statement;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
use std::time::Instant;
/// Main entry point for the SQL REPL.
///
/// Creates a database instance and enters an interactive loop where users
/// can enter SQL statements. Each statement is parsed, analyzed, prepared,
/// and executed in sequence.
///
fn analyze(db: &Database, stmt: ASTStmt) -> std::result::Result<Box<dyn Statement>, SemanticError> {
    let s = SemanticAnalysis::new(db);
    s.analyze(stmt.clone())
}

// fn debug_db(db: &Database) {
//     println!("num of tables in db {}", db.tables.len());
//
//     for (table_name, _) in db.tables.iter() {
//         println!(
//             "table {} has {} columns",
//             table_name,
//             db.get_table(table_name).unwrap().columns.len()
//         );
//         println!(
//             "table {} has {} records",
//             table_name,
//             db.get_table(table_name).unwrap().data.len()
//         );
//     }
// }

fn run_sql(db: &mut Database, sql: &str) {
    let parser = Parser::default();
    let ast = match parser.parse(sql) {
        Ok(ast) => ast,
        Err(e) => {
            println!("Startup SQL parse error: {e:#?}");
            return;
        }
    };

    for stmt in ast.statements {
        let mut st = match analyze(db, stmt) {
            Ok(st) => {
                if std::env::var("IMLAB_PRINT_CODE")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false)
                {
                    println!("{:#?}", st);
                }
                st
            }
            Err(e) => {
                println!("Error in semantic analysis: {e}");
                continue;
            }
        };

        if let Err(e) = st.prepare(db) {
            println!("Error in prepare statement: {e}");
            continue;
        }

        let mut stdout = std::io::stdout();
        if let Err(e) = st.execute(db, &mut stdout) {
            println!("Error in execute statement: {e}");
            continue;
        }
    }
}

pub fn main() -> Result<()> {
    let mut db = Database::default();

    const STARTUP_SQL: [&str; 6] = [
        "CREATE TABLE test (col1 INTEGER NOT NULL, col2 INTEGER NOT NULL);",
        "CREATE TABLE test2 (col1 VARCHAR(20) NOT NULL, col2 INTEGER NOT NULL);",
        "CREATE TABLE test3 (col1 VARCHAR(20) NOT NULL, col2 INTEGER NOT NULL);",
        "COPY test FROM 'data/test.tbl' DELIMITER '|';",
        "COPY test3 FROM 'data/test.tbl' DELIMITER '|';",
        "COPY test2 FROM 'data/test2.tbl';",
    ];

    fn run_startup(db: &mut Database) {
        for query in STARTUP_SQL {
            run_sql(db, query);
        }
    }

    let mut rl = DefaultEditor::new()?;
    #[cfg(feature = "with-file-history")]
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }
    loop {
        // debug_db(&db);
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                // Ignore empty input
                if trimmed.is_empty() {
                    continue;
                }

                match trimmed {
                    // REPL meta-commands
                    "startup_test" => {
                        run_startup(&mut db);
                        rl.add_history_entry(line.as_str())?;
                    }
                    "startup_tpcc" => {
                        let start = Instant::now();
                        run_sql_file(&mut db, "data/tpcc_schema.sql");
                        run_sql_file(&mut db, "data/copy_tpcc.sql");
                        let duration = start.elapsed();
                        println!("Loaded TPCC in {:?}", duration);
                        rl.add_history_entry(line.as_str())?;
                    }

                    "startup_imdb" => {
                        run_sql_file(&mut db, "data/my_imdb_schema.sql");
                        rl.add_history_entry(line.as_str())?;
                    }
                    "help" => {
                        println!(
                            "Commands:\nstartup - run predefined startup SQL\nhelp - show this message\nexit - exit REPL"
                        );
                        rl.add_history_entry(line.as_str())?;
                    }
                    "exit" => break,
                    // SQL
                    _ => {
                        rl.add_history_entry(line.as_str())?;
                        run_sql(&mut db, &line);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    #[cfg(feature = "with-file-history")]
    rl.save_history("history.txt");
    Ok(())
}

fn run_sql_file(db: &mut Database, path: &str) {
    match std::fs::read_to_string(path) {
        Ok(contents) => run_sql(db, &contents),
        Err(e) => println!("Could not read startup SQL file '{}': {}", path, e),
    }
}

// create table warehouse (
// w_id integer not null,
// w_name varchar(10) not null,
// w_street_1 varchar(20) not null,
// w_street_2 varchar(20) not null,
// w_city varchar(20) not null,
// w_state char(2) not null,
// w_zip char(9) not null,
// w_tax numeric(4, 4) not null,
// w_ytd numeric(12, 2) not null,
// primary key (w_id)
// );
// COPY warehouse FROM 'data/tpcc/warehouse.tbl' DELIMITER '|';
// Create table test (col1 INTEGER NOT NULL, col2);
// Create table test (col1 INTEGER NOT NULL, col2 INTEGER);
// Create table test (col1 INTEGER NOT NULL, col2 INTEGER NOT NULL );
//
// Create table test2 (col1 VARCHAR(20) NOT NULL, col2 INTEGER NOT NULL );
//
// COPY test FROM 'bin/test.tbl' DELIMITER '|';
//
// select * from test;
//
// select col1 from test;
//
// select col1 from test, test2;
//
// select t2.col1 from test, test2 t2;
//
// select t2.col1 from test t2, test2 t2;
//
// select t2.col1 from test, test2 t2;
//
// select test.col1 from test t;
//
// select col123 from test;
//
// select test.col123 from test;
//
// select * from test, test2 t2 where t2.col1 = 5;

// Create table test (col1 INTEGER NOT NULL, col2 INTEGER NOT NULL );
// Create table test2 (col1 VARCHAR(20) NOT NULL, col2 INTEGER NOT NULL );
// COPY test FROM 'bin/test.tbl' DELIMITER '|';
// COPY test2 FROM 'bin/test2.tbl';
// select * from test t, test2 t2;
//select * from history where h_c_id = 1 AND h_c_d_id = 1 AND h_c_w_id = 1 AND h_d_id = 4;

// select t.col1, t2.col1 from test t, test2 t2 where t.col2 = t2.col2;
