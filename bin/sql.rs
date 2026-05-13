//! SQL REPL Binary
//!
//! Implement a SQL Read-Eval-Print Loop (REPL) that provides an
//! interactive interface for executing SQL DDL and DQL statements.

use colored::Colorize;
use imlab::db::Database;
use imlab::parser::Parser;
use imlab::parser::ast::Statement as ASTStmt;
use imlab::semana::SemanticAnalysis;
use imlab::semana::semana_errors::SemanticError;
use imlab::statement::Statement;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
use std::time::{Duration, Instant};
use indexmap::IndexMap;

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

fn run_sql(db: &mut Database, sql: &str) {
    let parser = Parser::default();
    let ast = match parser.parse(sql) {
        Ok(ast) => ast,
        Err(e) => {
            println!("{}", format!("Error when parsing: {e:#?}").red());
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
                println!("{}", format!("Error in semantic analysis: {e}").red());
                continue;
            }
        };

        if let Err(e) = st.prepare(db) {
            println!("{}", format!("Error in prepare statement: {e}").red());
            continue;
        }

        let mut stdout = std::io::stdout();
        if let Err(e) = st.execute(db, &mut stdout) {
            println!("{}", format!("Error in execute statement: {e}").red());
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
                    "benchmark_imdb" => {
                        run_sql_file(&mut db, "data/my_imdb_schema.sql");
                        rl.add_history_entry(line.as_str())?;
                        benchmark_file(&mut db, "data/imdb_queries_phf.sql");
                        rl.add_history_entry(line.as_str())?;
                    }

                    "help" => {
                        println!(
                            "Commands:\n\
                            startup_test - run predefined startup SQL\n\
                            startup_tpcc - run predefined startup tpcc SQL\n\
                            startup_imdb - run predefined startup imdb SQL\n\
                            benchmark_imdb - run predefined startup imdb SQL\n\
                            help - show this message\n\
                            exit - exit REPL"
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

fn benchmark_file(db: &mut Database,path: &str) {
    let sql = std::fs::read_to_string(path).unwrap();

    let stmts: Vec<_> = sql.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let mut map: IndexMap<i32, Duration> = IndexMap::new();

    let mut query_num = 0;
    for stmt in stmts{
        let start = Instant::now();
        for _ in (0..6){
            run_sql(db, &*(stmt.to_owned() + ";"));
        }
        let total_time = start.elapsed();
        map.insert(query_num, total_time/6);
        query_num += 1;
    }

    for (query_num, time) in map{
        println!("query_{} avg time: {:?}", query_num, time);
    }
}
