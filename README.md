# My DB

## How to run a small test
```shell
RUSTFLAGS=-Awarnings cargo run --bin sql --release
CREATE TABLE test (col1 INTEGER NOT NULL, col2 INTEGER NOT NULL);
CREATE TABLE test2 (col1 VARCHAR(20) NOT NULL, col2 INTEGER NOT NULL);
COPY test FROM 'data/test.tbl' DELIMITER '|';
COPY test2 FROM 'data/test2.tbl';
SELECT * FROM test;
SELECT col1 FROM test;
SELECT * FROM test WHERE test.col1 = 1;
SELECT * FROM test t WHERE t.col1 = 1;
SELECT * FROM test t, test2 t2 WHERE t.col2 = t2.col2;
SELECT * FROM test t, test2 t2 WHERE t.col2 = t2.col2 AND t.col2 = 123;
SELECT * FROM test , (SELECT h_c_id FROM history) WHERE h_c_id = test.col2;
```


## How to run 
### 1.Imdb Benchmark:
- Store the dataset files in the `data/imdb` folder. 
- Start the repl with: 
```shell
RUSTFLAGS=-Awarnings cargo run --bin sql --release
```
to run in debug mode ( print the generated code ):
```shell
RUSTFLAGS=-Awarnings IMLAB_PRINT_CODE=true cargo run --bin sql --release
```
- Run `startup_imdb` to create the tables and load the data.
- Exit with `exit`
## Task 1

1. Implement row or column store for [your database](src/db/mod.rs). Create the TPC-C tables specified in
   [schema.sql](data/tpcc_schema.sql) in [tpcc.rs](bin/tpcc.rs). Use the types specified [here](src/types). Add appropriate
   data structures for the primary keys in your database!
2. Implement load functions in [tpcc.rs](bin/tpcc.rs) to parse TBL files (= CSV files) and populate your tables with the
   data of [these TPC-C files](https://db.in.tum.de/teaching/ws2122/imlab/tpcc_5w.tar.gz) (please do not push them into
   the GitLab). Do not forget the primary keys!
3. Implement the [New Order](data/new_order.sql) transaction as a Rust function in [tpcc.rs](bin/tpcc.rs).
4. Add and extend the tests for your implementation [here](bin/tpcc.rs#L168-205).

How many New Order transactions per second can your implementation execute?

## Task 2

1. Implement a SQL parser using [lrpar](https://crates.io/crates/lrpar) and [lrlex](https://crates.io/crates/lrlex)
   * The parser takes the text input and returns an [AST](src/parser/ast.rs) struct
   * The `AST` should consist of a vector of SQL statements
   * The parser must be able to parse the DDL statements of [schema.sql](data/tpcc_schema.sql) following the grammar
     specified below
   * The parser must also support the parsing of copy statements
2. Implement [semantic analysis](src/semana/mod.rs) for the AST of the parser
   * The semantic analysis takes an `AST` statement and returns an implementation of [Statement](src/statement/mod.rs)
   * Return a meaningful error if a table of that name already exists in the database
   * Return a meaningful error if attributes are not declared `NOT NULL` or are declared twice
3. Implement a Read-Evaluate-Print-Loop (REPL) in [bin/sql.rs](bin/sql.rs):
   * Read one line from standard in
   * Run the parser on the input
   * For each statement of the `AST`:
      * Run the semantic analysis on the parsed AST statement, and call `prepare` and `execute` on the returned statement
      * In case of a [CreateStatement](src/statement/create.rs), create the table with its columns in the database.
      * In case of a [CopyStatement](src/statement/copy.rs), populate the tables with the data of the specified data
        file. Reinterpret your generic type for the data depending on the column type(s) and parse the table data
        using the input function of the type. It is enough to support only the TPC-C schema
   * Loop

This is the grammar for `CREATE` and `COPY`:

```
CREATE TABLE table_name '('
    [
        column_name data_type NOT NULL
        [',' column_name data_type NOT NULL]...
        [',' PRIMARY_KEY '(' column_name [',' column_name]... ')']
    ]
')';

COPY table_name FROM file [DELIMITER delimiter];

(where file and delimiter are single quoted string literals, e.g. 'foo')
```

The skeleton for this task contains a [lexemes](src/parser/foo.l) and a [grammar](src/parser/foo.y) file for a toy
format `FOO` which looks like this:

```
foo bar1 {
    a integer,
    b char{20}
};

foo bar2 {
    c integer
};
```

Take a look at [the parser](src/parser/mod.rs), the [lexemes](src/parser/foo.l) and the [grammar](src/parser/foo.y) file
as well as [the documentation for lrpar and lrlex](https://softdevteam.github.io/grmtools/latest_release/book/index.html)
to understand how they work together.

Replace the toy files with a new lexemes file `sql.l` and a grammar file `sql.y` and adapt the parser as needed. Remember to
update [build.rs](build.rs)!

Bonus tasks:

* Can you populate your tables with the TPC-C data in less than 1s?

## Task 3

1. Extend the existing parser to also parse queries for the following subset of SQL DQL statements:
   ```
   SELECT { '*' | [column_name [',' column_name]...] }
   FROM table_name [',' table_name]...
   [ WHERE column_name '=' { column_name | constant } [ AND column_name '=' { column_name | constant } ]...];

   table_name := text text (e.g. students s) | text (e.g. students)
   column_name := text.text (e.g. s.matrnr) | text (e.g. matrnr)
   ```

   * The select clause contains one or more attribute names
   * The from clause contains one or more relation names
   * The where clause is optional and can contain one or more selections (connected by "and")
   * You only need to support selections of the form "attribute = attribute" and "attribute = constant"
2. Extend the semantic analysis for DQL statements:
   * Build an algebra tree for the DQL statement and return it inside the [SelectStatement](src/statement/select.rs)
   * Use the [Print operator](src/algebra/print.rs) for the select clause, the [Join operator](src/algebra/join.rs) and
     [TableScan operator](src/algebra/scan.rs) for the from clause, the [Select operator](src/algebra/select.rs) for
     the where clause
   * Report errors when non-existing attributes or tables are referenced
   * Build your tree from bottom up: First analyze the from clause, then the where clause, and finally the select
     clause. Build left-deep join trees based on the order of the relations in the from clause
3. Extend the REPL in [sql.rs](bin/sql.rs) if needed

Bonus tasks:

* Implement parsing and semantic analysis for subqueries in the from clause, i.e., `'(' SELECT ... ')' [ subquery_name ]`
  as an alternative to `table_name`

## Task 4

1. Implement code generation from relational algebra trees to Rust code using the [produce/consume (push) model](https://db.in.tum.de/teaching/ss24/moderndbs/chapter6.pdf)
    * You need to support the following operators: [TableScan](src/algebra/scan.rs), [Select](src/algebra/select.rs), [Print](src/algebra/print.rs) and [Join](src/algebra/join.rs)
    * To evaluate expressions, implement `produce` on [Expression](src/parser/expr.rs)
2. Implement code generation, compilation and execution in the [SelectStatement](src/statement/select.rs):
    1. Call `prepare` and `produce` on the operator tree and write the generated code into a temporary file
    2. Compile the temporary file with the Rust compiler to a shared library (.so or .dylib file), e.g., `rustc --crate-type dylib /tmp/code.rs`
    3. Load the shared library using `dlopen`, `dlsym`, and `dlclose` and execute the query. You are allowed to use a wrapper crate like [dlopen2](https://crates.io/crates/dlopen2)

Bonus tasks:

* Implement code generation for the [CopyStatement](src/statement/copy.rs)

## Task 5

1. Implement the [LazyMultiMap](src/infra/map.rs) hashtable
2. Implement predicate push down as an optimization pass for your algebra tree, i.e., shift all select operators as far
   down in the tree as possible, and into joins where applicable
3. Implement hash join for join operators having binary expressions:
    * Consume the left input of the join into a LazyMultiMap hashtable
    * Pass the right input to the parent operator if there is a match in the hashtable

Bonus tasks:

* Support more expressions, like addition

## Task 6

* Implement [morsel-driven parallelism](https://dl.acm.org/doi/pdf/10.1145/2588555.2610507) in your query engine:
   * Use [rayon](https://crates.io/crates/rayon) to parallelize data processing
   * Use `par_iter` in your table scans
   * Make your hash table of task 5 thread-safe and build it multi-threaded
