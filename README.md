# My DB

## Overview

This is an in-memory, row-store database written in Rust with an interactive SQL REPL that does JIT compilation for SELECT queries.

Supports only Linux and MacOS.

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
startup_tpcc // special command to load the table from data/tpcc/
SELECT * FROM test , (SELECT h_c_id FROM history) h WHERE h.h_c_id = test.col2;
exit
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

You can run the TPCC new order test: 
```shell
RUSTFLAGS=-Awarnings cargo run --bin sql --release
```

This is the grammar for `CREATE`, `COPY` and `SELECT`:

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

SELECT { '*' | [column_name [',' column_name]...] }
FROM table_name [',' table_name]...
[ WHERE column_name '=' { column_name | constant } [ AND column_name '=' { column_name | constant } ]...];

table_name := text text (e.g. students s) | text (e.g. students)
column_name := text.text (e.g. s.matrnr) | text (e.g. matrnr)

'(' SELECT ... ')' [ subquery_name ]
```

### More technical stuff
- Generates rust code.
- Uses morsel-driven parallelism to execute queries efficiently ( parallelizes TableScan operators, LazyMultiMap and Print Operators without locks).
- Can throw these [errors](src/semana/semana_errors.rs) 
