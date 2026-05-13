# IMLAB: Technical Writeup

## Project Overview

IMLAB is a high-performance, in-memory row-store relational database engine written entirely in Rust. It is designed to execute SQL queries with a focus on JIT (Just-In-Time) code generation for SELECT queries, enabling dynamic query compilation to native machine code for optimal performance.

**Key Characteristics:**
- In-memory, row-based storage model
- Interactive SQL REPL for query execution
- JIT-compiled query execution for zero-interpretation overhead
- Support for standard SQL features (CREATE TABLE, COPY, SELECT)
- Relational algebra-based query optimization
- Type-safe schema management

---

## Architectural Overview

IMLAB follows a classical database architecture composed of distinct layers, each with clear responsibilities:

```
┌─────────────────────────────────────────────────────────┐
│          SQL REPL (bin/sql.rs)                          │
│          - Interactive query interface                  │
│          - Query execution orchestration                │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│          Parser Layer (parser/)                         │
│          - Lexical analysis (sql.l)                     │
│          - Syntactic analysis (sql.y)                   │
│          - AST generation                               │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│          Semantic Analysis (semana/)                    │
│          - Type checking                                │
│          - Reference resolution                         │
│          - Statement validation                         │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│          Statement Execution (statement/)               │
│          - CREATE TABLE                                 │
│          - COPY (data loading)                          │
│          - SELECT (query execution)                     │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼───────────────────┐
│ Relational Algebra Layer (algebra/)  │
│ - TableScan              │ - Join    │
│ - Select                 │ - Print   │
└──────────────────────────────────────┘
       │
┌──────▼──────────────────────────────────┐
│   Code Generation (codegen/)            │
│   - Produces Rust code                  │
│   - Compiles to shared library          │
│   - Dynamically loads executable        │
└─────────────────────────────────────────┘
       │
┌──────▼──────────────────────────────────┐
│   Storage Engine (db/)                  │
│   - Row-store implementation            │
│   - Type system (types/)                │
│   - Index management                    │
└─────────────────────────────────────────┘
```

---

## Core Components

### 1. **Parser Layer** (`src/parser/`)

The parser layer converts SQL text into an Abstract Syntax Tree (AST) using industry-standard LR parsing.

**Components:**
- **sql.l** - Lexer definition (lrlex format)
  - Tokenizes SQL input
  - Case-insensitive keyword handling
  - String literal and numeric literal recognition
  
- **sql.y** - Parser grammar (lrpar/grmtools format)
  - Defines grammar rules for DDL and DQL
  - Generates conflict-free LR parser tables
  - Builds strongly-typed AST nodes
  
- **ast.rs** - Intermediate representation
  - Statement enum (CreateTable, CopyTable, Select)
  - Table, Column, and CreateTable definitions
  - Primary key specification
  
- **expr.rs** - Expression representation
  - Binary expressions for WHERE clauses
  - Column references and constants
  - Type-safe expression tree structure

**Supported SQL Syntax:**
```sql
-- DDL
CREATE TABLE table_name (
    column_name data_type NOT NULL,
    PRIMARY KEY (col1, col2)
);

-- Data Loading
COPY table_name FROM 'file.tbl' [DELIMITER 'delimiter'];

-- DQL (with limited WHERE clause)
SELECT [*|col1, col2, ...] 
FROM table_name [alias]
[WHERE predicate AND predicate ...]
```

### 2. **Semantic Analysis Layer** (`src/semana/`)

The semantic analysis phase validates the AST and builds executable statements. This layer performs:

**Key Operations:**
- **Type Checking** - Ensures expressions match column types
- **Reference Resolution** - Validates table and column names exist
- **Schema Validation** - Checks constraints like primary keys
- **Scope Management** - Tracks table aliases and column namespaces using Scope objects

**SemanticAnalysis::analyze()** flow:
1. Validates CREATE TABLE statements (no duplicates, unique columns, valid constraints)
2. Validates COPY statements (file existence, table existence)
3. Validates SELECT statements (builds relational algebra tree)
   - Analyzes FROM clause (table references, aliases)
   - Analyzes WHERE clause (predicate extraction and column resolution)
   - Tracks all Internal Units (IUs) - logical column references

**Output:** Strongly-typed Statement objects ready for execution

### 3. **Relational Algebra Layer** (`src/algebra/`)

The algebra layer represents query execution plans as trees of operators using the **Volcano model** for query processing.

**Operators Implemented:**

#### a) **TableScan**
- Reads rows from base tables
- Generates code to iterate over all rows
- Uses parallel iteration with MORSEL-sized chunks (data morsels for cache efficiency)
- Output: Exposes all columns from the table as IUs

**Code Generation Pattern:**
```rust
// Parallelized iteration with chunks for cache locality
(0..table.num_rows).into_par_iter()
    .chunks(MORSEL_SIZE)
    .for_each(|morsel| {
        for row_id in morsel {
            // Process each row
        }
    })
```

#### b) **Select (Filter)**
- Applies WHERE clause predicates
- Pushes predicates down the tree during optimization
- Evaluates expressions that reference columns
- Output: Subset of input rows that satisfy conditions

#### c) **Join**
- Implements equijoin for combining multiple tables
- **Join Strategy:** Hash join (build_left/probe_right)
  - Build phase: Consumes left child, constructs hash table indexed by join keys
  - Probe phase: Consumes right child, probes hash table for matches
- Supports CROSS JOIN (no predicates) and INNER JOIN (with predicates)
- Uses LazyMultiMap for efficient hash aggregation

**Join Implementation Details:**
- Keys are extracted from join predicates
- Hash table maps serialized key bytes → row IDs
- Row IDs are stored compactly (as vectors) to avoid materializing full rows
- Access templates enable on-demand row reconstruction during probe phase

**Parallel Insertion & Atomic Operations:**
- Leverages `LazyMultiMapParBuilder` for thread-safe hash table construction
- Build phase uses thread-local buffering via `ThreadLocal<RefCell<Vec<Entry>>>`
  - Each thread accumulates entries in its own buffer without synchronization overhead
  - Eliminates contention during parallel data collection from left child
- During hash table finalization, parallel insertion phase uses **Compare-And-Swap (CAS)** atomics:
  ```rust
  loop {
      let head = table[bucket_idx].load(Ordering::Relaxed);
      entry.next = head;
      
      // CAS: atomically update bucket head if it hasn't changed
      match table[bucket_idx].compare_exchange_weak(
          head,
          entry_ptr,
          Ordering::Relaxed,
          Ordering::Relaxed,
      ) {
          Ok(_) => break,      // Successfully inserted
          Err(_) => continue,  // Retry if another thread modified head
      }
  }
  ```
- **Benefits of this approach:**
  - Multiple threads can build separate entries in parallel without global locking
  - CAS creates lock-free linked list insertion into hash buckets
  - Relaxed memory ordering adequate since synchronization happens at finalization boundary
  - Efficiently handles high-cardinality joins with skewed key distributions

#### d) **Print**
- Root operator that formats and outputs results
- Consumes tuples from children
- Formats columns according to type specifications
- Tracks row counts for metrics

**Operator Pattern (Volcano Model):**
```rust
pub trait Operator {
    fn produce(&self, codegen: &mut Codegen) -> Result<(), Box<dyn Error>>;
    fn consume(&self, codegen: &mut Codegen, caller, tuple_ctx) -> Result<(), Box<dyn Error>>;
    fn ius(&self) -> HashSet<IURef>;  // Internal Units (columns)
    fn optimize(&self, pass: OptimizerPass);
}
```

### 4. **Type System** (`src/types/`)

IMLAB implements a compact, bit-packed type system for memory efficiency and performance.

**Type Encoding (64-bit word):**
```
Bits 63-56: Type Tag (8 bits)
Bits 55-24: Precision (32 bits) 
Bits 23-8:  Scale (16 bits)
Bit 0-7:    Flags (nullable, etc.)
```

**Supported Types:**
- **INTEGER** - 32-bit signed integers
- **BOOLEAN** - Single-bit booleans
- **NUMERIC(p,s)** - Fixed-point decimals (precision p, scale s)
  - Max precision: 38 digits
  - Max scale: 38 digits
- **TEXT** - Variable-length UTF-8 strings (unbounded)
- **VARCHAR(n)** - Variable-length strings (max length n)
- **CHAR(n)** - Fixed-length character arrays
- **TIMESTAMP** - Date/time values

**Value Encoding:**
- Fixed-size types encoded inline (INTEGER, BOOLEAN)
- Variable-length types stored separately with offset indirection
- NULL values tracked via nullable flag in type

### 5. **Storage Engine** (`src/db/`)

The storage engine implements a row-store layout with emphasis on cache efficiency and compact representation.

#### Table Storage:
```
┌─────────────────────────────┐
│   Table Structure           │
├─────────────────────────────┤
│ columns: Vec<Column>        │ Schema information
├─────────────────────────────┤
│ raw_data: Vec<u8>           │ Contiguous row bytes
│ row_size: usize             │ Bytes per row
│ num_rows: usize             │ Row count
├─────────────────────────────┤
│ col_offsets: Vec<usize>     │ Byte offset of each column
│ key_index: FastMap[K→RowID] │ Primary key index (hash map)
│ key_positions: Vec<usize>   │ Which columns are keys
└─────────────────────────────┘
```

**Data Layout (Row-Store):**
```
Row 0:   [Col0 bytes][Col1 bytes][Col2 bytes]...
Row 1:   [Col0 bytes][Col1 bytes][Col2 bytes]...
Row n:   [Col0 bytes][Col1 bytes][Col2 bytes]...
```

**Column Access:**
- Calculate row pointer: `row_id * row_size`
- Add column offset: `row_ptr + col_offset`
- Decode value from bytes according to type

**Index Structure:**
- FastMap (ahash-based HashMap) for primary keys
- Keys serialized as byte vectors for bitwise comparison
- Maps key bytes → row ID for O(1) lookups

### 6. **Infrastructure Layer** (`src/infra/`)

The infrastructure layer provides low-level utilities optimized for parallel query execution.

**LazyMultiMap & LazyMultiMapParBuilder:**
- **LazyMultiMap** - Lock-free multi-value hash table with chaining collision resolution
  - Entry storage: Contiguous vector for cache locality
  - Bucket management: Linked lists via pointer chaining
  - Lookup: Linear scan through chain for matching keys
  
- **LazyMultiMapBuilder** - Single-threaded sequential builder
  - Simple batch insertion into vector
  - Fast finalization via parallel CAS-based table construction
  
- **LazyMultiMapParBuilder** - Thread-safe parallel builder with CAS finalization
  - Thread-local buffering: Each thread accumulates entries isolation (**zero locks**)
    ```rust
    pub struct LazyMultiMapParBuilder<K, V> {
        locals: ThreadLocal<RefCell<Vec<LazyMultiMapEntry<K, V>>>>,
    }
    ```
  - Design tradeoff: Single synchronization point at finalization vs. continuous contention-free insertion
  - Parallel finalization using atomic CAS operations (Ordering::Relaxed) for lock-free linked list insertion
  - Enables efficient multi-threaded hash table construction for join operations

**Why CAS + Atomic Pointers?**
- Standard Rust mutexes would serialize all insertions
- CAS-based approach allows all threads to insert concurrently
- Ordering::Relaxed sufficient since synchronization occurs at finalization boundary
- Atomicity guarantees correct linked-list structure even with concurrent modifications

### 7. **Code Generation and JIT** (`src/codegen/`)

The code generation layer is the engine that enables dynamic query compilation. It transforms relational algebra expression trees into compilable Rust code.

**Compilation Pipeline:**
```
Operator Tree
     ↓
produce()/consume() codegen calls
     ↓
Generated Rust code (String buffer)
     ↓
Write to temp file (e.g., query_0.rs)
     ↓
rustc compilation to shared library
     ↓
dlopen() dynamic loading
     ↓
Function pointer extraction
     ↓
Execute as native code
```

**Code Generation Strategy:**
1. **Top-down traversal** via `produce()` - Each operator describes how to generate rows
2. **Bottom-up code materialization** via `consume()` - Parent operators consume tuples from children
3. **Context passing** - TupleCtx struct tracks:
   - Variable expressions for each IU (column)
   - Row ID information for reconstruction
   - Expressions needed for parents

**Generated Code Structure:**
```rust
#[no_mangle] pub extern "C" fn query_0(db: *const Database) -> i32 {
    let db = unsafe { &*db };
    
    // Preamble: Table and column variable declarations
    let table_0 = &db.tables.get("users").unwrap();
    let col_0 = 0; // Column index for 'id'
    let col_1 = 1; // Column index for 'name'
    
    let mut rows = 0i32;
    
    // Function body: Nested loops from bottom-up execution
    (0..table_0.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each(|chunk| {
        for row_id in chunk {
            let view = table_0.row_view(row_id);
            
            // Filter predicate (from Select operator)
            if let Some(id_val) = view.get(col_0) {
                if id_val == (10 as i32).to_le_bytes() {
                    // Print/consume tuple
                    println!("{:?}", view.get(col_1));
                    rows.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    });
    
    rows
}
```

**Key Design Decisions:**
1. **Parallel Execution** - Uses rayon's parallel iterators with morsel-based chunking
2. **MORSEL_SIZE** - Cache-line aligned data morsel size for prefetching efficiency
3. **Lazy Compilation** - Code generated on-demand per query
4. **Unsafe Rust** - Minimal use of unsafe for FFI and database pointer dereferencing
5. **Thread-local Storage** - Some operations use thread-local caches for concurrent queries

---

## Query Execution Flow

### Step 1: Parsing
```
Input: "SELECT col1 FROM test WHERE col1 = 5;"
                    ↓
                 Lexer (sql.l)
                    ↓
        Tokens: [SELECT, col1, FROM, test, WHERE, col1, =, 5]
                    ↓
                Parser (sql.y)
                    ↓
            AST(Query { 
                targets: [ColId("col1")], 
                from: [TableFactor("test")],
                where: [BinaryExpr(col1, Eq, 5)]
            })
```

### Step 2: Semantic Analysis
```
AST Input
    ↓
Validate table "test" exists
Validate column "col1" exists in table
Create Internal Units (IUs) for each referenced column
Build scope mapping: "col1" → IU_ref_1
Analyze WHERE predicate, extract join/filter conditions
    ↓
SelectStatement {
    op: Rc<Print(
        child: Rc<Select(
            child: Rc<TableScan("test")>,
            predicates: [col1 = 5]
        )>
    )>
}
```

### Step 3: Preparation (Optimization & Planning)
```
Operator tree
    ↓
prepare() phase - each operator initializes
    ↓
optimize(PredicatePushdown) - push filters down
    ↓
Second prepare() - operators recalculate after optimization
    ↓
Final tree ready for code generation
```

### Step 4: Code Generation
```
Operator tree root (Print)
    ↓
Print.produce() calls Child.produce()
    ↓
Select.produce() calls Child.produce()
    ↓
TableScan.produce() generates:
    - Table/column declarations
    - Parallel iteration loops
    - Column value extraction
    ↓
TableScan.consume() called by Select
    - Select generates filter check
    - Builds TupleCtx for Parent
    ↓
Select.consume() called by Print
    - Select passes filtered tuples up
    ↓
Print.consume()
    - Generates output formatting code
    - Increments row counter
    ↓
Generated Rust code string produced
```

### Step 5: Compilation & Loading
```
Generated code
    ↓
Write to temporary .rs file
    ↓
Spawn rustc process (release mode)
    ↓
Compile to shared library (.so/.dylib)
    ↓
dlopen() to load into process memory
    ↓
dlsym() to extract function pointer
    ↓
Cast to JitMainFn type
```

### Step 6: Execution
```
JitMainFn function pointer
    ↓
Call unsafe extern "C" function
    ↓
Native optimized code runs
    ↓
Result: row count returned
    ↓
Metrics printed:
    - Compilation time
    - Execution time
```

---

## Implementation Details

### 1. Internal Units (IUs)

IUs represent logical columns in a query. They flow through the operator tree carrying meaning.

```rust
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct IURef {
    pub table_var: String,  // e.g., "t0"
    pub name: String,       // e.g., "col1"
}
```

**IU Propagation:**
- TableScan produces IUs for all scanned columns
- Each operator tracks which IUs it needs from children
- Code generation uses IU references to generate variable names

### 2. TupleCtx - Tuple Context

TupleCtx carries tuple-related information during code generation traversal.

```rust
pub struct TupleCtx {
    pub exprs: HashMap<IURef, String>,           // IU → Rust expression
    pub access: HashMap<IURef, (String, String)>, // IU → (table_var, col_idx_var)
    pub row_ids_expr: String,                    // Variable holding row IDs
    pub row_id_tables: Vec<String>,              // Order of tables
}
```

**Example:**
```
For SELECT t.col1, u.col2 FROM t JOIN u ON t.id = u.id:

IU_t_col1 → TupleCtx {
    exprs: { IU_t_col1 → "view_0.get(col_0).unwrap()" },
    access: { IU_t_col1 → ("table_0", "0") }
}
```

### 3. Data Type Representation

Types are compact 64-bit encodings enabling efficient metadata tracking:

```rust
pub struct Type {
    code: u64,  // All type info in single word
}

// Bit layout:
// 63-56:  Tag (Type::Integer, Type::Varchar, etc.)
// 55-24:  Precision (length for strings, precision for numeric)
// 23-8:   Scale (for NUMERIC types)
// 0:      Nullable flag
```

**Why 64-bit encoding?**
- Single word type can be stored inline
- Fast to pass by value
- Cache-friendly for large schema arrays
- Enables atomic type operations

### 4. Hash Join Implementation

Hash joins follow the build/probe paradigm optimized for in-memory execution with parallel insertion support:

**Build Phase (left child consumed in parallel):**
```rust
// Thread-local buffering - no lock contention
let builder = LazyMultiMapParBuilder::new();
left_rows.par_iter().for_each(|row_id| {
    let key = extract_key_bytes(row_id, left_schema);
    builder.insert(key, row_id);  // Inserts into thread-local buffer
});

// Finalization: merge buffers and build hash table with CAS
let hash_map = builder.finalize();  // Parallel CAS-based insertion
```

**Finalization with Parallel CAS Insertion:**
- Entries from all thread-local buffers are merged into a single vector
- Hash table buckets initialized as `AtomicPtr<Entry>` for lock-free updates
- Each entry is processed in parallel via rayon's `par_iter_mut()`
- Per-entry insertion uses Compare-And-Swap for atomic linked-list prepend:
  ```rust
  entries.par_iter_mut().for_each(|entry| {
      let bucket_idx = hash(entry.key) & mask;
      
      loop {
          // Load current head (Relaxed: within finalization barrier)
          let head = table[bucket_idx].load(Ordering::Relaxed);
          entry.next = head;
          
          // CAS: atomically update if head unchanged
          if table[bucket_idx].compare_exchange_weak(
              head,                 // expected
              entry as *mut _,      // new value
              Ordering::Relaxed,    // success
              Ordering::Relaxed     // failure
          ).is_ok() {
              break;  // Success
          }
          // Else: retry if concurrent modification detected
      }
  });
  ```

**Probe Phase (right child consumed in parallel):**
```rust
for row_id in right_rows {
    let key = extract_key_bytes(row_id, right_schema);
    if let Some(left_row_ids) = hash_map.get(&key) {
        for left_row_id in left_row_ids {
            emit_joined_tuple(left_row_id, row_id);
        }
    }
}
```

### 5. Parallel Execution Strategy

Queries execute in parallel using Rayon's work-stealing scheduler:

```rust
// Morsel-driven parallelism
(0..table.num_rows)
    .into_par_iter()           // Parallel iterator
    .chunks(MORSEL_SIZE)       // Group into cache-friendly chunks
    .for_each(|morsel| {
        for row_id in morsel {
            // Process row
        }
    })
```

**Benefits:**
- Efficient core utilization
- Better cache locality (MORSEL_SIZE ≈ L2/L3 cache line)
- Automatic load balancing via work-stealing
- Locks avoided through thread-local aggregation

---

## Key Technologies & Dependencies

### Core Libraries:
- **lrlex + lrpar** (v0.13) - LR parser generators (Rust version of GNU bison)
- **libloading** (v0.9) - Dynamic library loading for JIT-compiled code
- **rayon** (v1.11) - Parallel computing framework
- **hashbrown** (v0.16) + **ahash** (v0.8) - Fast hashing and HashMap implementation
- **csv** (v1.2) - Efficient CSV parsing for data loading
- **rustyline** (v17) - Line editing for REPL

### Supporting Libraries:
- **anyhow** + **thiserror** - Error handling
- **enum_dispatch** - Efficient trait dispatch without vtables
- **colored** - Terminal output coloring
- **phf** (Perfect Hash Functions) - Compile-time string mapping

### Build System:
- **cfgrammar** + **lrlex** build dependencies for grammar compilation
- Standard Rust build toolchain (cargo, rustc)

---

## Performance Characteristics

### Advantages:
1. **JIT Compilation** - Zero interpretation overhead, full CPU optimization
2. **Memory Layout** - Row-store with compact encoding minimizes memory bandwidth
3. **Parallelization** - Query operators automatically parallelized via Rayon
4. **Hash Joins** - In-memory hash tables with linear probing (ahash)
5. **Type System** - 64-bit type encoding enables efficient schema representation

### Optimization Techniques Implemented:
1. **Predicate Pushdown** - SELECT operators pushed below JOINs
2. **Morsel-Driven Parallelism** - Cache-line sized chunks processed per thread
3. **Lazy Expression Evaluation** - Row materialization deferred until needed
4. **Hash Aggregation** - HashMap-based join without sorting

### Current Limitations:
1. **In-Memory Only** - No persistence or spilling to disk
2. **No Query Planner** - Relational algebra tree built directly from semantic analysis
3. **Limited WHERE Clauses** - Only equality predicates and AND combinations
4. **Single-Table Scans** - Predicate pushdown only, scan fusion not implemented
5. **No Cost-Based Optimization** - No statistics or cardinality estimates

---

## Code Organization & Module Structure

```
src/
├── lib.rs                  # Library root, module declarations
├── algebra/
│   ├── mod.rs             # Operator trait and Op enum
│   ├── iu.rs              # Internal Unit (column reference) implementation
│   ├── scan.rs            # TableScan operator
│   ├── select.rs          # Select (filter) operator
│   ├── join.rs            # Join operator (hash join implementation)
│   └── print.rs           # Print operator (output)
├── codegen/
│   └── mod.rs             # Rust code generation + compilation pipeline
├── db/
│   ├── mod.rs             # Database root
│   ├── database.rs        # Database (table management)
│   ├── table.rs           # Table (row-store implementation)
│   ├── column.rs          # Column metadata
│   ├── row.rs             # Row serialization/deserialization
│   └── value.rs           # Value type enum
├── parser/
│   ├── mod.rs             # Parser orchestration
│   ├── ast.rs             # Abstract syntax tree definitions
│   ├── expr.rs            # Expression (WHERE clause) representation
│   ├── sql.l              # Lexer (lrlex format)
│   └── sql.y              # Grammar (lrpar/grmtools format)
├── semana/
│   ├── mod.rs             # Semantic analysis main logic
│   ├── scope.rs           # Scope management for name resolution
│   └── semana_errors.rs   # Semantic error types
├── statement/
│   ├── mod.rs             # Statement trait
│   ├── create.rs          # CREATE TABLE statement
│   ├── copy.rs            # COPY (data loading) statement
│   └── select.rs          # SELECT statement
├── types/
│   ├── mod.rs             # Type system definition
│   ├── integer.rs         # Integer type implementation
│   ├── text.rs            # Text/Varchar type
│   ├── numeric.rs         # Numeric/Decimal type
│   ├── bool.rs            # Boolean type
│   └── timestamp.rs       # Timestamp type
├── infra/
│   ├── mod.rs             # Infrastructure utilities
│   └── map.rs             # LazyMultiMap (hash table utilities)
└── util/
    ├── mod.rs             # Utilities
    └── io.rs              # I/O helpers

bin/
├── sql.rs                 # SQL REPL binary
├── create_table.rs        # Utility for DDL execution
└── tpcc.rs                # TPCC benchmark driver
```

---

## Future Enhancement Opportunities

### Short-term:
1. **Query Optimizer** - Cost-based SQL optimizer with selectivity estimation
2. **Aggregate Functions** - SUM, COUNT, AVG, GROUP BY support
3. **Additional Operators** - UNION, INTERSECT, EXCEPT
4. **Extended Predicates** - LIKE, IN, BETWEEN, range predicates
5. **Explain Plan** - Query plan visualization

### Medium-term:
1. **Indexing** - B-tree indices for range queries, bitmap indices
2. **Query Caching** - Memoization of compiled queries
3. **Sort Support** - ORDER BY, merge sort for large datasets
4. **Aggregation** - Hash aggregation, streaming aggregation
5. **Window Functions** - Analytical functions with OVER clause

### Long-term:
1. **Persistence** - Page-based storage with transaction support
2. **MVCC** - Multi-version concurrency control
3. **Distributed** - Sharding and distributed execution
4. **Advanced Types** - JSON, arrays, custom UDTs
5. **Machine Learning** - Integration with ML libraries

---

## Testing & Benchmarking

### Test Suites:
- **Parser tests** - Grammar coverage and error handling
- **Type tests** - Encoding/decoding correctness
- **Database tests** - Table operations and schema validation
- **Integration tests** - End-to-end query execution

### Benchmarks:
- **TPCC** - TPC-C transactional benchmark (in `bin/tpcc.rs`)
- **IMDB** - IMDb dataset queries (in `data/imdb_queries_phf.sql`)
- **Custom** - User-supplied test cases

### Performance Metrics:
- Compilation time (code generation + rustc)
- Execution time (JIT function execution)
- Memory usage (table size, intermediate structures)
- Row throughput (rows processed per millisecond)

---

## Conclusion

IMLAB demonstrates a sophisticated, modern approach to relational database engineering:

1. **Elegant Architecture** - Clear separation of concerns with well-defined interfaces
2. **Contemporary Techniques** - JIT compilation, morsel-driven parallelism, hash joins
3. **Type Safety** - Rust's type system prevents entire classes of runtime errors
4. **Performance Focus** - Every layer designed with performance in mind
5. **Educational Value** - Serves as reference implementation for database internals

The codebase exemplifies how modern programming languages and techniques can be leveraged to build high-performance data systems that are both functionally correct and maintainable.

---

**Author's Note:**
This database engine was developed as an educational project to explore modern database system design patterns. It serves as a working reference for understanding query optimization, code generation, and in-memory execution strategies in contemporary database systems.

