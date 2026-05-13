//! SELECT statement implementation.

use crate::algebra::iu::IURef;
use crate::algebra::{Op, Operator, OptimizerPass};
use crate::codegen::{Codegen, JitMainFn};
use crate::db::Database;
use crate::statement::Statement;
use colored::Colorize;
use std::error::Error;
use std::io::Write;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Executable SELECT statement for query processing.
#[derive(Debug)]
pub struct SelectStatement {
    /// The relational algebra operator tree representing the query plan
    op: Rc<Op>,
    generated_code: String,
    comp_time: Duration,
    main_fn: Option<JitMainFn>,
}

impl SelectStatement {
    pub fn new(op: Rc<Op>) -> Self {
        Self {
            op,
            generated_code: String::new(),
            comp_time: Duration::new(0, 0),
            main_fn: None,
        }
    }
}

impl Statement for SelectStatement {
    /// Prepares the SELECT statement for execution.
    fn prepare(&mut self, _db: &mut Database) -> Result<(), Box<dyn Error>> {
        let required_ius = std::collections::HashSet::<IURef>::new();
        self.op.prepare(&self.op, required_ius)?;

        self.op.optimize(OptimizerPass::PredicatePushdown);

        let required_ius = std::collections::HashSet::<IURef>::new();
        self.op.prepare(&self.op, required_ius)?;

        let mut codegen = Codegen::new();
        self.op.produce(&mut codegen)?;

        self.generated_code = codegen.text().to_string();

        // debug
        if std::env::var("IMLAB_PRINT_CODE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            println!("----- Generated Rust Code -----");
            println!("{}", self.generated_code);
            println!("--------------------------------");
        }

        let start = Instant::now();
        codegen.compile_and_load()?;
        self.main_fn = codegen.main_fn;
        let duration = start.elapsed();
        self.comp_time = duration;

        Ok(())
    }

    /// Executes the SELECT statement.
    fn execute(&mut self, db: &mut Database, _out: &mut dyn Write) -> Result<(), Box<dyn Error>> {
        // - Execute the compiled function
        if self.main_fn.is_none() {
            self.prepare(db)?;
        }

        let main_fn = self
            .main_fn
            .ok_or("JIT function not loaded (prepare() did not succeed)")?;

        let start = Instant::now();
        let _ = unsafe { main_fn(db)? };
        let duration = start.elapsed();

        println!(
            "{} {}",
            "Compilation time is :".dimmed(),
            format!("{:?}", self.comp_time).dimmed()
        );

        println!(
            "{} {}",
            "Execution time is :".dimmed(),
            format!("{:?}", duration).dimmed()
        );

        Ok(())
    }
}
