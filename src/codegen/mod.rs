//! Code generation for compiling relational algebra trees to executable Rust code.

use crate::db::Database;
use libloading::Library;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Type alias for the main function of generated query code. Change as you like.
pub(crate) type JitMainFn = unsafe extern "C" fn(&Database) -> Result<(), Box<dyn Error>>;
static QUERY_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Code generator that builds executable Rust code from relational algebra trees.
///
/// The Codegen struct accumulates generated code fragments from operators using
/// the produce/consume model, then compiles the complete code to a shared library
/// and dynamically loads it for execution.
#[derive(Default)]
pub struct Codegen {
    buf: String,
    indent: usize,
    tmp_counter: usize,
    pub(crate) main_fn: Option<JitMainFn>,
    /// Loaded shared library (must be kept alive while calling the function)
    lib: Option<Library>,
    lib_path: Option<PathBuf>,
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
        self.indent = 0;
    }

    #[inline]
    pub fn line<S: AsRef<str>>(&mut self, s: S) {
        for _ in 0..self.indent {
            self.buf.push_str("    ");
        }
        self.buf.push_str(s.as_ref());
        self.buf.push('\n');
    }

    #[inline]
    pub fn same_line<S: AsRef<str>>(&mut self, s: S) {
        for _ in 0..self.indent {
            self.buf.push_str("    ");
        }
        self.buf.push_str(s.as_ref());
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
        self.open(format!(
            "#[no_mangle] pub extern \"C\" fn {}(db: &imlab::db::Database) -> Result<(), Box<dyn Error>>",
            name
        ));
        // self.line("let mut out_count: usize = 0;");
    }

    pub fn end_function(&mut self) {
        self.line("Ok(())");
        self.close();
    }

    pub fn new_var(&mut self, prefix: &str) -> String {
        let name = format!("{}{}", prefix, self.tmp_counter);
        self.tmp_counter += 1;
        name
    }

    /// Compiles the generated code to an executable function.
    ///
    /// Takes the accumulated code fragments, assembles them into a complete
    /// Rust program, compiles it using the Rust compiler, and dynamically
    /// loads the resulting shared library.
    ///
    /// # Returns
    /// * `Ok(JitMainFn)` - Dynamically loaded function ready for execution
    /// * `Err(...)` - Compilation or loading error
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
            .expect("could not find libimlab*.rlib in target/debug/deps");

        let status = Command::new("rustc")
            .arg("--crate-type")
            .arg("dylib")
            .arg("--edition=2021")
            .arg("-Awarnings") // suppress warnings
            // Tell rustc where to find *dependencies* of imlab (enum_dispatch, etc.)
            .arg("-L")
            .arg(format!("dependency={}", deps_dir))
            // Link imlab itself
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

        // Ensure the Library is dropped so the OS releases any locks
        self.lib = None;

        if let Some(lib_path) = &self.lib_path {
            // Ignore errors during cleanup
            let _ = fs::remove_file(lib_path);

            // Try to infer the source file name from the lib filename: libimlab_query_{id}.dylib / .so
            if let Some(fname) = lib_path.file_name().and_then(|n| n.to_str()) {
                if fname.starts_with("libimlab_query_") {
                    // strip prefix
                    let without_prefix = &fname["libimlab_query_".len()..];
                    // remove known suffixes
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
        cg.end_function();

        // println!("{}", cg.buf);
        println!("{}", cg.text());
    }
    // #[ignore] // too expensive for CI
    // fn test_codegen() {
    //     let mut c = Codegen::default();
    //     // TODO: Generate some code, compile, execute and assert correctness
    // }
}
