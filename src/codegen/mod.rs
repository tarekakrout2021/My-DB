//! Code generation for compiling relational algebra trees to executable Rust code.

use crate::db::Database;
use libloading::Library;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Type alias for the main function of generated query code.
pub(crate) type JitMainFn = unsafe extern "C" fn(*const Database) -> i32;

static QUERY_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Code generator that builds executable Rust code from relational algebra trees.
///
/// The Codegen struct accumulates generated code fragments from operators using
/// the produce/consume model, then compiles the complete code to a shared library
/// and dynamically loads it for execution.
#[derive(Default)]
pub struct Codegen {
    buf: String,

    /// Code inside the current function body (excluding preamble).
    func_body: String,

    /// Function-scope declarations that must be visible everywhere in the function.
    func_preamble: Vec<String>,

    /// Are we currently emitting inside a function body?
    in_function: bool,

    indent: usize,
    tmp_counter: usize,

    table_map: HashMap<String, String>,         // table_name -> table_var
    col_map: HashMap<(String, String), String>, // (table_var, col_name) -> col_var

    pub(crate) main_fn: Option<JitMainFn>,

    /// Loaded shared library (must be kept alive while calling the function)
    lib: Option<Library>,
    lib_path: Option<PathBuf>,

    pub map_names: HashSet<String>,
}

impl Codegen {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn text(&self) -> &str {
        &self.buf
    }

    #[inline]
    pub fn clear(&mut self) {
        self.buf.clear();
        self.func_body.clear();
        self.func_preamble.clear();
        self.table_map.clear();
        self.col_map.clear();
        self.in_function = false;
        self.indent = 0;
        self.tmp_counter = 0;
    }

    #[inline]
    fn push_line_to(target: &mut String, indent: usize, s: &str) {
        for _ in 0..indent {
            target.push_str("    ");
        }
        target.push_str(s);
        target.push('\n');
    }

    #[inline]
    pub fn line<S: AsRef<str>>(&mut self, s: S) {
        if self.in_function {
            Self::push_line_to(&mut self.func_body, self.indent, s.as_ref());
        } else {
            Self::push_line_to(&mut self.buf, self.indent, s.as_ref());
        }
    }

    #[inline]
    pub fn same_line<S: AsRef<str>>(&mut self, s: S) {
        let target = if self.in_function {
            &mut self.func_body
        } else {
            &mut self.buf
        };

        for _ in 0..self.indent {
            target.push_str("    ");
        }
        target.push_str(s.as_ref());
    }

    #[inline]
    pub fn open<S: AsRef<str>>(&mut self, s: S) {
        self.line(format!("{} {{", s.as_ref()));
        self.indent += 1;
    }

    #[inline]
    pub fn close(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
        self.line("}");
    }

    /// Convenience for the root `Print` to start/finish a function.
    pub fn start_function(&mut self, name: &str) {
        self.func_body.clear();
        self.func_preamble.clear();
        self.table_map.clear();
        self.col_map.clear();

        self.open(format!(
            "#[no_mangle] pub extern \"C\" fn {}(db: *const imlab::db::Database) -> i32",
            name
        ));
        self.in_function = true;

        self.preamble_line("if db.is_null() { return -1; }");
        self.preamble_line("let db = unsafe { &*db };");
    }

    pub fn end_function(&mut self) {
        self.in_function = false;

        for l in self.func_preamble.drain(..) {
            Self::push_line_to(&mut self.buf, 1, &l);
        }

        self.buf.push_str(&self.func_body);
        self.func_body.clear();

        Self::push_line_to(&mut self.buf, 1, "rows as i32");

        self.close();
    }

    pub fn new_var(&mut self, prefix: &str) -> String {
        let name = format!("{}{}", prefix, self.tmp_counter);
        self.tmp_counter += 1;
        name
    }

    #[inline]
    pub fn preamble_line<S: Into<String>>(&mut self, s: S) {
        self.func_preamble.push(s.into());
    }

    pub fn get_and_declare_table(&mut self, table_name: &str) -> String {
        if let Some(v) = self.table_map.get(table_name) {
            return v.clone();
        }

        let var = self.new_var("table_");
        self.preamble_line(format!(
            "let {} = db.get_table(\"{}\").unwrap();",
            var, table_name
        ));
        self.table_map.insert(table_name.to_string(), var.clone());
        var
    }

    pub fn get_and_declare_col(&mut self, table_var: &str, col_name: &str) -> String {
        let key = (table_var.to_string(), col_name.to_string());
        if let Some(v) = self.col_map.get(&key) {
            return v.clone();
        }

        let var = self.new_var("col_");
        self.preamble_line(format!(
            "let {} = {}.get_col_index(\"{}\").unwrap() as usize;",
            var, table_var, col_name
        ));
        self.col_map.insert(key, var.clone());
        var
    }

    /// Compiles the generated code to an executable function.
    ///
    /// Takes the accumulated code fragments, assembles them into a complete
    /// Rust program, compiles it using the Rust compiler, and dynamically
    /// loads the resulting shared library.
    pub(crate) fn compile_and_load(&mut self) -> Result<(), Box<dyn Error>> {
        use std::fs;
        use std::process::Command;

        let (src_path, lib_path) = Self::temp_paths();

        if std::env::var("IMLAB_PRINT_CODE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            eprintln!("DEBUG: writing generated source to {}", src_path.display());
        }

        fs::write(&src_path, &self.buf)?;

        let deps_dir = if cfg!(debug_assertions) {
            "target/debug/deps"
        } else {
            "target/release/deps"
        };

        let rlib_path = fs::read_dir(deps_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("libimlab") && n.ends_with(".rlib"))
                    .unwrap_or(false)
            })
            .expect("could not find libimlab*.rlib in target/{debug,release}/deps");

        let status = Command::new("rustc")
            .arg("--crate-type")
            .arg("cdylib")
            .arg("--edition=2021")
            .arg("-Awarnings")
            .arg("-L")
            .arg(format!("dependency={}", deps_dir))
            .arg("--extern")
            .arg(format!("imlab={}", rlib_path.display()))
            .arg(&src_path)
            .arg("-o")
            .arg(&lib_path)
            .status()?;

        if !status.success() {
            return Err(format!("rustc failed with status: {}", status).into());
        }

        let lib = unsafe { Library::new(&lib_path)? };

        unsafe {
            let func: libloading::Symbol<JitMainFn> = lib.get(b"main_query")?;
            self.main_fn = Some(*func);
        }

        self.lib = Some(lib);
        self.lib_path = Some(lib_path);

        Ok(())
    }

    fn temp_paths() -> (PathBuf, PathBuf) {
        let id = QUERY_COUNTER.fetch_add(1, Ordering::Relaxed);

        let mut src = std::env::temp_dir();
        src.push(format!("imlab_query_{id}.rs"));

        let mut lib = std::env::temp_dir();

        #[cfg(target_os = "macos")]
        lib.push(format!("libimlab_query_{id}.dylib"));

        #[cfg(target_os = "linux")]
        lib.push(format!("libimlab_query_{id}.so"));

        (src, lib)
    }
}

impl Drop for Codegen {
    fn drop(&mut self) {
        use std::fs;

        self.lib = None;

        if let Some(lib_path) = &self.lib_path {
            let _ = fs::remove_file(lib_path);

            if let Some(fname) = lib_path.file_name().and_then(|n| n.to_str()) {
                if fname.starts_with("libimlab_query_") {
                    let without_prefix = &fname["libimlab_query_".len()..];
                    let id_str = without_prefix
                        .trim_end_matches(".dylib")
                        .trim_end_matches(".so");

                    let mut src = std::env::temp_dir();
                    src.push(format!("imlab_query_{}.rs", id_str));
                    let _ = fs::remove_file(src);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_close_indent() {
        let mut cg = Codegen::new();
        cg.line("use imlab::db::Database;");
        cg.start_function("foo");
        cg.open("for i in 0..10");
        cg.open("for j in 0..10000");
        cg.line("println!(\"{}{}\", i, j);");
        cg.close();
        cg.close();
        cg.line("let rows = 0usize;");
        cg.end_function();

        println!("{}", cg.text());
    }
}