//! TPC-C Benchmark Implementation
//!
//! Implement the New Order transaction of the TPC-C (Transaction Processing Performance
//! Council - C) in this binary. Add data structures in `Database` to store schema
//! information and table data, create the tables for TPC-C here, load the data
//! from the CSV files, translate the transaction from SQL to Rust, and run it!
//!
//! How many transactions can you run per second?

mod create_table;
use create_table::create_tpcc_tables;

extern crate core;

use std::error::Error;
use std::time::{Duration, Instant};

use rand::Rng;

use imlab::db::row::ValueRef;
use imlab::db::{Database, Key, Row, Table, Value};
use imlab::types::Type;
use imlab::types::prelude::*;

type ResultAny<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Number of warehouses in the TPC-C dataset
const WAREHOUSES: usize = 5;

/// Wrapper for the TPC-C related functions.
#[derive(Default)]
#[allow(clippy::upper_case_acronyms)]
struct TPCC;

impl TPCC {
    /// Loads the TPC-C tables from CSV files in the specified directory.
    ///
    /// Expected files: warehouse.tbl, district.tbl, customer.tbl, history.tbl,
    /// neworder.tbl, order.tbl, orderline.tbl, item.tbl, stock.tbl
    ///

    pub fn load(db: &mut Database, base_path: &str) -> ResultAny<()> {
        let tables_meta = vec![
            ("warehouse", "warehouse.tbl"),
            ("district", "district.tbl"),
            ("customer", "customer.tbl"),
            ("history", "history.tbl"),
            ("neworder", "neworder.tbl"),
            ("order", "order.tbl"),
            ("orderline", "orderline.tbl"),
            ("item", "item.tbl"),
            ("stock", "stock.tbl"),
        ];

        let mut handles = Vec::new();

        for (name, file) in tables_meta {
            let schema = db
                .get_table(name)
                .ok_or_else(|| anyhow::anyhow!("Table {} not found", name))?
                .empty_like();

            let path = format!("{}/{}", base_path, file);
            let name_owned = name.to_string();

            handles.push(std::thread::spawn(move || -> ResultAny<(String, Table)> {
                let mut table = schema;
                table
                    .load_from_file(&path, "|")
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                Ok((name_owned, table))
            }));
        }

        for handle in handles {
            let (name, loaded_table) = handle
                .join()
                .map_err(|_| anyhow::anyhow!("Thread panic"))??;
            *db.tables
                .get_mut(&name)
                .ok_or_else(|| anyhow::anyhow!("Table {} missing", name))? = loaded_table;
        }

        Ok(())
    }

    /// Generates and executes a random New Order transaction.
    fn random_new_order(db: &mut Database) -> Result<Duration, Box<dyn Error>> {
        let mut rng = rand::rng();

        let mut nu_rng = rand::rng();
        let mut nu_rand = |a, x, y| {
            (((nu_rng.random_range(0..a + 1) | nu_rng.random_range(x..y + 1)) + 42) % (y - x + 1))
                + x
        };

        let w_id = Integer::new(rng.random_range(1i32..(WAREHOUSES + 1) as i32));
        let d_id = Integer::new(rng.random_range(1..11));
        let c_id = Integer::new(nu_rand(1023, 1, 3000));
        let ol_cnt = Integer::new(rng.random_range(5..16));

        let mut supware = [Integer::new(0); 15];
        let mut itemid = [Integer::new(0); 15];
        let mut qty = [Integer::new(0); 15];

        let ts = Timestamp::new(0);

        for i in 0..i32::from(ol_cnt) as usize {
            let supware_item = if rng.random_range(1..101) > 1 {
                w_id
            } else {
                let mut r = rng.random_range(1..(WAREHOUSES - 1) as i32);
                r += (r >= i32::from(w_id)) as i32;
                Integer::new(r)
            };

            supware[i] = supware_item;
            itemid[i] = Integer::new(nu_rand(8191, 1, 100000));
            qty[i] = Integer::new(rng.random_range(1..11));
        }

        // TODO: Retrieve the actual tables from the database
        // let mut tables = vec![];

        let now = Instant::now();
        TPCC::new_order(db, w_id, d_id, c_id, ol_cnt, &supware, &itemid, &qty, ts)?;
        let elapsed = now.elapsed();

        Ok(elapsed)
    }

    /// Executes a New Order transaction.
    #[allow(clippy::too_many_arguments)]
    fn new_order(
        db: &mut Database,
        w_id: Integer,
        d_id: Integer,
        c_id: Integer,
        items: Integer,
        supware: &[Integer; 15],
        itemid: &[Integer; 15],
        qty: &[Integer; 15],
        datetime: Timestamp,
    ) -> Result<(), Box<dyn Error>> {
        const W_TAX: usize = 7;

        const C_DISCOUNT: usize = 15;

        const D_TAX: usize = 8;
        const D_NEXT_O_ID: usize = 10;

        const I_PRICE: usize = 3;

        // stock: s_i_id(0), s_w_id(1), s_quantity(2), s_dist_01..10(3..12), s_ytd(13), s_order_cnt(14), s_remote_cnt(15), s_data(16)
        const S_QUANTITY: usize = 2;
        const S_DIST_01: usize = 3;
        const S_ORDER_CNT: usize = 14;
        const S_REMOTE_CNT: usize = 15;

        // -------- small helpers (inline-ish, no allocations) --------
        #[inline]
        fn vref_i32(v: ValueRef) -> Result<i32, Box<dyn Error>> {
            match v {
                ValueRef::Integer(i) => Ok(i),
                _ => Err("Expected Integer".into()),
            }
        }

        #[inline]
        fn vref_u64(v: ValueRef) -> Result<u64, Box<dyn Error>> {
            match v {
                ValueRef::Numeric(n) => Ok(n),
                _ => Err("Expected Numeric".into()),
            }
        }

        #[inline]
        fn numeric_to_f64(raw: u64, scale: u32) -> f64 {
            raw as f64 / 10u64.pow(scale) as f64
        }

        #[inline]
        fn f64_to_numeric(raw: f64, scale: u32) -> Numeric {
            let mul = 10u64.pow(scale) as f64;
            Numeric::new((raw * mul).round() as u64)
        }

        #[inline]
        fn i32_to_numeric0(x: i32) -> Result<Numeric, Box<dyn Error>> {
            if x < 0 {
                return Err("Negative value into numeric(?,0)".into());
            }
            Ok(Numeric::new(x as u64))
        }

        let w_id_i = w_id.value;
        let d_id_i = d_id.value;
        let c_id_i = c_id.value;
        let items_usize = items.value as usize;

        // -------- 1) warehouse tax (numeric(4,4)) --------
        let w_tax = {
            let t = db
                .get_table("warehouse")
                .ok_or("warehouse table not found")?;
            let key: Key = vec![Integer::new(w_id_i).into()];
            let rv = t.get_view(&key).ok_or("warehouse row not found")?;
            numeric_to_f64(vref_u64(rv.get(W_TAX).ok_or("w_tax missing")?)?, 4)
        };

        // -------- 2) customer discount (numeric(4,4)) --------
        let c_discount = {
            let t = db.get_table("customer").ok_or("customer table not found")?;
            let key: Key = vec![
                Integer::new(w_id_i).into(),
                Integer::new(d_id_i).into(),
                Integer::new(c_id_i).into(),
            ];
            let rv = t.get_view(&key).ok_or("customer row not found")?;
            numeric_to_f64(
                vref_u64(rv.get(C_DISCOUNT).ok_or("c_discount missing")?)?,
                4,
            )
        };

        // -------- 3) district read o_id + d_tax; 4) bump next_o_id via put_field --------
        let district_key: Key = vec![Integer::new(w_id_i).into(), Integer::new(d_id_i).into()];

        let (o_id_i, d_tax) = {
            let t = db.get_table("district").ok_or("district table not found")?;
            let rv = t.get_view(&district_key).ok_or("district row not found")?;
            let o_id_i = vref_i32(rv.get(D_NEXT_O_ID).ok_or("d_next_o_id missing")?)?;
            let d_tax = numeric_to_f64(vref_u64(rv.get(D_TAX).ok_or("d_tax missing")?)?, 4);
            (o_id_i, d_tax)
        };

        {
            let t = db
                .get_table_mut("district")
                .ok_or("district table not found (mut)")?;
            t.put_field(
                &district_key,
                D_NEXT_O_ID,
                &Value::Integer(Integer::new(o_id_i + 1)),
            )?;
        }

        // -------- 5) all_local (numeric(1,0)) --------
        let all_local_num = {
            let mut all_local = 1i32;
            for idx in 0..items_usize {
                if supware[idx].value != w_id_i {
                    all_local = 0;
                    break;
                }
            }
            i32_to_numeric0(all_local)?
        };

        // -------- 6) insert order (still via Row for now) --------
        {
            let t = db
                .get_table_mut("order")
                .ok_or("\"order\" table not found")?;
            let row: Row = vec![
                Value::Integer(Integer::new(o_id_i)),
                Value::Integer(Integer::new(d_id_i)),
                Value::Integer(Integer::new(w_id_i)),
                Value::Integer(Integer::new(c_id_i)),
                Value::Timestamp(datetime),
                Value::Integer(Integer::new(0)), // carrier_id
                Value::Numeric(i32_to_numeric0(items.value)?), // o_ol_cnt numeric(2,0)
                Value::Numeric(all_local_num),   // o_all_local numeric(1,0)
            ];
            t.insert(row)?;
        }

        // -------- 7) insert neworder --------
        {
            let t = db
                .get_table_mut("neworder")
                .ok_or("neworder table not found")?;
            let row: Row = vec![
                Value::Integer(Integer::new(o_id_i)),
                Value::Integer(Integer::new(d_id_i)),
                Value::Integer(Integer::new(w_id_i)),
            ];
            t.insert(row)?;
        }

        // -------- 8) loop items: reads via get_view, updates via put_field --------
        for idx in 0..items_usize {
            let supply_w_i = supware[idx].value;
            let i_id_i = itemid[idx].value;
            let ol_qty_i = qty[idx].value;

            // item price numeric(5,2)
            let i_price = {
                let t = db.get_table("item").ok_or("item table not found")?;
                let key: Key = vec![Value::Integer(Integer::new(i_id_i))];
                let rv = t.get_view(&key).ok_or("item row not found")?;
                numeric_to_f64(vref_u64(rv.get(I_PRICE).ok_or("i_price missing")?)?, 2)
            };

            // stock key is (s_w_id, s_i_id) in your code/index
            let stock_key: Key = vec![
                Value::Integer(Integer::new(supply_w_i)),
                Value::Integer(Integer::new(i_id_i)),
            ];

            // read stock values zero-copy
            let (s_qty_i, s_order_cnt_i, s_remote_cnt_i, s_dist_str) = {
                let t = db.get_table("stock").ok_or("stock table not found")?;
                let rv = t.get_view(&stock_key).ok_or("stock row not found")?;

                let s_qty_i = vref_u64(rv.get(S_QUANTITY).ok_or("s_quantity missing")?)? as i32;
                let s_order_cnt_i =
                    vref_u64(rv.get(S_ORDER_CNT).ok_or("s_order_cnt missing")?)? as i32;
                let s_remote_cnt_i =
                    vref_u64(rv.get(S_REMOTE_CNT).ok_or("s_remote_cnt missing")?)? as i32;

                if !(1..=10).contains(&d_id_i) {
                    return Err("d_id out of range (expected 1..=10)".into());
                }
                let dist_col = S_DIST_01 + (d_id_i as usize - 1);
                let dist_v = rv.get(dist_col).ok_or("s_dist_XX missing")?;
                let tv = match dist_v {
                    ValueRef::Text(tv) => tv,
                    _ => return Err("Expected Text for s_dist_XX".into()),
                };

                (
                    s_qty_i,
                    s_order_cnt_i,
                    s_remote_cnt_i,
                    tv.as_str().to_owned(),
                )
            };

            // compute new quantity
            let new_qty = if s_qty_i > ol_qty_i {
                s_qty_i - ol_qty_i
            } else {
                s_qty_i + 91 - ol_qty_i
            };

            // update stock in-place with put_field (no Vec<Value>)
            {
                let t = db
                    .get_table_mut("stock")
                    .ok_or("stock table not found (mut)")?;

                t.put_field(
                    &stock_key,
                    S_QUANTITY,
                    &Value::Numeric(i32_to_numeric0(new_qty)?),
                )?;

                t.put_field(
                    &stock_key,
                    S_ORDER_CNT,
                    &Value::Numeric(i32_to_numeric0(s_order_cnt_i + 1)?),
                )?;

                if supply_w_i != w_id_i {
                    t.put_field(
                        &stock_key,
                        S_REMOTE_CNT,
                        &Value::Numeric(i32_to_numeric0(s_remote_cnt_i + 1)?),
                    )?;
                }
            }

            // ol_amount numeric(6,2)
            let ol_amount_f64 =
                (ol_qty_i as f64) * i_price * (1.0 + w_tax + d_tax) * (1.0 - c_discount);

            // insert orderline
            {
                let t = db
                    .get_table_mut("orderline")
                    .ok_or("orderline table not found")?;
                let dist_text = Text::input(&s_dist_str, Type::new_char(24))?;

                let row: Row = vec![
                    Value::Integer(Integer::new(o_id_i)),
                    Value::Integer(Integer::new(d_id_i)),
                    Value::Integer(Integer::new(w_id_i)),
                    Value::Integer(Integer::new((idx + 1) as i32)),
                    Value::Integer(Integer::new(i_id_i)),
                    Value::Integer(Integer::new(supply_w_i)),
                    Value::Timestamp(Timestamp::new(0)), // NOT NULL in schema
                    Value::Numeric(i32_to_numeric0(ol_qty_i)?), // numeric(2,0)
                    Value::Numeric(f64_to_numeric(ol_amount_f64, 2)), // numeric(6,2)
                    Value::Text(dist_text),
                ];
                t.insert(row)?;
            }
        }

        Ok(())
    }
}

/// Main entry point for the TPC-C benchmark program.
pub fn main() -> Result<(), Box<dyn Error>> {
    // number of transactions to execute
    const TRANSACTIONS: usize = 1_000_000;

    // the database
    let mut db = Database::default();

    create_tpcc_tables(&mut db)?;

    let now = Instant::now();
    TPCC::load(&mut db, "data/tpcc").map_err(|e| e as Box<dyn Error>)?;
    let elapsed = now.elapsed();

    println!("Loaded TPC-C data in {}ms", elapsed.as_millis());

    let mut elapsed = Duration::default();
    for _ in 0..TRANSACTIONS {
        elapsed += TPCC::random_new_order(&mut db)?;
    }

    let throughput = TRANSACTIONS as f64 / elapsed.as_secs_f64();
    let elapsed = elapsed.as_millis();
    println!(
        "Executed {TRANSACTIONS} transactions in {elapsed}ms, that's {throughput} transactions per second!"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use imlab::db::Key;

    #[test]
    fn test_creation() -> Result<(), Box<dyn Error>> {
        use imlab::db::Key;
        use imlab::db::Row;
        use imlab::parser::ast::Column as ASTColumn;
        use imlab::parser::ast::Table as ASTTable;
        use imlab::types::Type;

        let table_id = String::from("1");

        let col1 = ASTColumn {
            id: String::from("col1"),
            r#type: Type::new_integer(),
            not_null: true,
        };
        let col2 = ASTColumn {
            id: String::from("col2"),
            r#type: Type::new_bool(),
            not_null: true,
        };
        let col3 = ASTColumn {
            id: String::from("col3"),
            r#type: Type::new_numeric(10, 10),
            not_null: true,
        };

        let ast_table = ASTTable {
            id: table_id.clone(),
            columns: vec![col1, col2, col3],
            primary_key: vec![String::from("col1"), String::from("col2")],
        };

        let row1: Row = vec![101.into(), true.into(), (1.123 as u64).into()];
        let row2: Row = vec![23.into(), false.into(), (1.123 as u64).into()];
        let row3: Row = vec![200.into(), false.into(), (1.123 as u64).into()];
        let row4: Row = vec![213.into(), false.into(), (1.123 as u64).into()];
        let row5: Row = vec![98.into(), false.into(), (1.123 as u64).into()];

        println!("{:?}", row4);

        let rows: Vec<Row> = vec![row1, row2, row3, row4, row5];

        let mut db = Database::default();
        db.create_table(&ast_table)?;

        let table = db.get_table_mut(&table_id).unwrap();
        for row in rows {
            table.insert(row)?;
        }

        let table = db.get_table(&table_id).unwrap();

        let key1: Key = vec![101.into(), true.into()];
        let key2: Key = vec![23.into(), false.into()];
        let key3: Key = vec![98.into(), true.into()];

        assert!(table.get_view(&key1).is_some(), "missing row for key1");
        assert!(table.get_view(&key2).is_some(), "missing row for key2");
        assert!(
            table.get_view(&key3).is_none(),
            "key3 should not exist (98,true) in inserted data"
        );

        println!("{:?}", table.get_view(&key1));

        Ok(())
    }
    #[ignore]
    #[test]
    fn load() -> Result<(), Box<dyn std::error::Error>> {
        let mut db = Database::default();
        create_tpcc_tables(&mut db)?;
        TPCC::load(&mut db, "data/tpcc").map_err(|e| e as Box<dyn Error>)?;

        fn table_len(db: &Database, name: &str) -> Result<usize, Box<dyn Error>> {
            Ok(db
                .get_table(name)
                .ok_or_else(|| format!("missing table '{name}'"))?
                .num_rows)
        }

        assert_eq!(table_len(&db, "customer")?, 150_000);
        assert_eq!(table_len(&db, "district")?, 50);
        assert_eq!(table_len(&db, "history")?, 150_000);
        assert_eq!(table_len(&db, "item")?, 100_000);
        assert_eq!(table_len(&db, "neworder")?, 45_000);
        assert_eq!(table_len(&db, "order")?, 150_000);
        assert_eq!(table_len(&db, "orderline")?, 1_425_564);
        assert_eq!(table_len(&db, "stock")?, 500_000);
        assert_eq!(table_len(&db, "warehouse")?, 5);

        // check for [no_o_id = 3001, no_d_id = 1, no_w_id = 4] in neworder
        let key1: Key = vec![4.into(), 10.into(), 3000.into()];
        let new_order_table = db.get_table("neworder").ok_or("neworder table not found")?;
        let row_id = new_order_table.get_row_id(&key1).unwrap();
        new_order_table.debug_row(row_id);

        let orderline_table = db
            .get_table("orderline")
            .ok_or("orderline table not found")?;

        orderline_table.debug_row(5);

        // if let Some(row) = db.get_table("neworder").unwrap().get_view(&key1) {
        //     println!("Row found : {:?}", row);
        // } else {
        //     panic!("Row not found");
        // }

        // todo: assert that o_ol_cnt = 4 for [o_o_id = 3001, o_d_id = 1, o_w_id = 4] in order

        Ok(())
    }

    // #[test]
    // fn insert() {
    //     // todo: insert the following tuples
    //     //  - [ol_o_id = 3001, ol_d_id = 1, ol_w_id = 4, ol_number = 1, ol_i_id = 2000, ol_supply_w_id = 2, ol_delivery_d = 0, ol_quantity = 2, ol_amount = 103.88, ol_dist_info = BNtvegAMKuMUjTFs4DsVSpcN]
    //     //  - [ol_o_id = 3001, ol_d_id = 1, ol_w_id = 4, ol_number = 2, ol_i_id = 4000, ol_supply_w_id = 3, ol_delivery_d = 0, ol_quantity = 4, ol_amount = 47.73, ol_dist_info = qPGVnI55KQLBPVsupnpx2DVv]
    //     //  - [ol_o_id = 3001, ol_d_id = 1, ol_w_id = 4, ol_number = 3, ol_i_id = 6000, ol_supply_w_id = 4, ol_delivery_d = 0, ol_quantity = 6, ol_amount = 287.71, ol_dist_info = KMBpiXpYSwaMYEYlN4FGtwsa]
    //     //  - [ol_o_id = 3001, ol_d_id = 1, ol_w_id = 4, ol_number = 4, ol_i_id = 8000, ol_supply_w_id = 1, ol_delivery_d = 0, ol_quantity = 8, ol_amount = 247.27, ol_dist_info = OGRIrXrkKCQkL2Y4aZILws20]
    //
    //     // todo: look up the inserted tuples and assert that ol_quantity is correct
    //
    //     todo!()
    // }
}
