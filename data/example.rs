// Example query
// SELECT COUNT(*)
// FROM
// (SELECT t.id, mc.company_id, mc.company_type_id
// FROM title t, movie_companies mc
// WHERE t.production_year >= 2000
// AND t.production_year <= 2003
// AND mc.id >= 1
// AND mc.id <= 350000
// AND t.id = mc.movie_id
// ) tm,
// aka_title at,
// movie_info mi,
// info_type it1,
// movie_keyword mk,
// keyword k,
// company_name cn,
// company_type ct
// WHERE at.id >= 1
// AND at.id <= 350000
// AND mi.id >= 1
// AND mi.id <= 350000
// AND mk.id >= 1
// AND mk.id <= 350000
// AND at.movie_id = tm.id
// AND mi.movie_id = tm.id
// AND mk.movie_id = tm.id
// AND it1.id = mi.info_type_id
// AND k.id = mk.keyword_id
// AND cn.id = tm.company_id
// AND ct.id = tm.company_type_id;

// ----- Generated Rust Code -----
// Print operator produce
extern crate imlab;
use std::error::Error;
use imlab::db::Database;
use imlab::db::Value;
use imlab::types::integer::Integer;
use imlab::types::text::Text;
use imlab::types::numeric::Numeric;
use imlab::types::timestamp::Timestamp;
use imlab::types::bool::Bool;
use imlab::db::Key;
use imlab::infra::map::LazyMultiMapParBuilder;
use imlab::db::row::RowView;
use imlab::db::row::ValueRef;
use imlab::rayon::prelude::*;
use imlab::algebra::print::ParOutput;
#[no_mangle] pub extern "C" fn main_query(db: &imlab::db::Database) -> Result<usize, Box<dyn Error>> {
    let mut out_count: usize = 0;
    let table_16 = db.get_table("title").unwrap();
    let col_21 = table_16.get_col_index("episode_nr").unwrap() as usize;
    let col_22 = table_16.get_col_index("episode_of_id").unwrap() as usize;
    let col_23 = table_16.get_col_index("id").unwrap() as usize;
    let col_24 = table_16.get_col_index("imdb_id").unwrap() as usize;
    let col_25 = table_16.get_col_index("imdb_index").unwrap() as usize;
    let col_26 = table_16.get_col_index("kind_id").unwrap() as usize;
    let col_27 = table_16.get_col_index("md5sum").unwrap() as usize;
    let col_28 = table_16.get_col_index("phonetic_code").unwrap() as usize;
    let col_29 = table_16.get_col_index("production_year").unwrap() as usize;
    let col_30 = table_16.get_col_index("season_nr").unwrap() as usize;
    let col_31 = table_16.get_col_index("series_years").unwrap() as usize;
    let col_32 = table_16.get_col_index("title").unwrap() as usize;
    let table_33 = db.get_table("movie_companies").unwrap();
    let col_38 = table_33.get_col_index("company_id").unwrap() as usize;
    let col_39 = table_33.get_col_index("company_type_id").unwrap() as usize;
    let col_40 = table_33.get_col_index("id").unwrap() as usize;
    let col_41 = table_33.get_col_index("movie_id").unwrap() as usize;
    let col_42 = table_33.get_col_index("note").unwrap() as usize;
    let table_46 = db.get_table("aka_title").unwrap();
    let col_51 = table_46.get_col_index("episode_nr").unwrap() as usize;
    let col_52 = table_46.get_col_index("episode_of_id").unwrap() as usize;
    let col_53 = table_46.get_col_index("id").unwrap() as usize;
    let col_54 = table_46.get_col_index("imdb_index").unwrap() as usize;
    let col_55 = table_46.get_col_index("kind_id").unwrap() as usize;
    let col_56 = table_46.get_col_index("md5sum").unwrap() as usize;
    let col_57 = table_46.get_col_index("movie_id").unwrap() as usize;
    let col_58 = table_46.get_col_index("note").unwrap() as usize;
    let col_59 = table_46.get_col_index("phonetic_code").unwrap() as usize;
    let col_60 = table_46.get_col_index("production_year").unwrap() as usize;
    let col_61 = table_46.get_col_index("season_nr").unwrap() as usize;
    let col_62 = table_46.get_col_index("title").unwrap() as usize;
    let table_66 = db.get_table("movie_info").unwrap();
    let col_71 = table_66.get_col_index("id").unwrap() as usize;
    let col_72 = table_66.get_col_index("info").unwrap() as usize;
    let col_73 = table_66.get_col_index("info_type_id").unwrap() as usize;
    let col_74 = table_66.get_col_index("movie_id").unwrap() as usize;
    let col_75 = table_66.get_col_index("note").unwrap() as usize;
    let table_79 = db.get_table("info_type").unwrap();
    let col_84 = table_79.get_col_index("id").unwrap() as usize;
    let col_85 = table_79.get_col_index("info").unwrap() as usize;
    let table_89 = db.get_table("movie_keyword").unwrap();
    let col_94 = table_89.get_col_index("id").unwrap() as usize;
    let col_95 = table_89.get_col_index("keyword_id").unwrap() as usize;
    let col_96 = table_89.get_col_index("movie_id").unwrap() as usize;
    let table_100 = db.get_table("keyword").unwrap();
    let col_105 = table_100.get_col_index("id").unwrap() as usize;
    let col_106 = table_100.get_col_index("keyword").unwrap() as usize;
    let col_107 = table_100.get_col_index("phonetic_code").unwrap() as usize;
    let table_111 = db.get_table("company_name").unwrap();
    let col_116 = table_111.get_col_index("country_code").unwrap() as usize;
    let col_117 = table_111.get_col_index("id").unwrap() as usize;
    let col_118 = table_111.get_col_index("imdb_id").unwrap() as usize;
    let col_119 = table_111.get_col_index("md5sum").unwrap() as usize;
    let col_120 = table_111.get_col_index("name").unwrap() as usize;
    let col_121 = table_111.get_col_index("name_pcode_nf").unwrap() as usize;
    let col_122 = table_111.get_col_index("name_pcode_sf").unwrap() as usize;
    let table_126 = db.get_table("company_type").unwrap();
    let col_131 = table_126.get_col_index("id").unwrap() as usize;
    let col_132 = table_126.get_col_index("kind").unwrap() as usize;
    let MORSEL_SIZE = 2048;
    let out = ParOutput::new();
    // Using LazyMultiMap
    let mut builder_0: LazyMultiMapParBuilder = LazyMultiMapParBuilder::new();
    // Build Phase
    // Using LazyMultiMap
    let mut builder_2: LazyMultiMapParBuilder = LazyMultiMapParBuilder::new();
    // Build Phase
    // Using LazyMultiMap
    let mut builder_4: LazyMultiMapParBuilder = LazyMultiMapParBuilder::new();
    // Build Phase
    // Using LazyMultiMap
    let mut builder_6: LazyMultiMapParBuilder = LazyMultiMapParBuilder::new();
    // Build Phase
    // Using LazyMultiMap
    let mut builder_8: LazyMultiMapParBuilder = LazyMultiMapParBuilder::new();
    // Build Phase
    // Using LazyMultiMap
    let mut builder_10: LazyMultiMapParBuilder = LazyMultiMapParBuilder::new();
    // Build Phase
    // Using LazyMultiMap
    let mut builder_12: LazyMultiMapParBuilder = LazyMultiMapParBuilder::new();
    // Build Phase
    // Using LazyMultiMap
    let mut builder_14: LazyMultiMapParBuilder = LazyMultiMapParBuilder::new();
    // Build Phase
    // TableScan
    (0..table_16.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | rows_17 | {
        for row_18 in rows_17  {
            let view_19 = RowView{ table: &table_16, row_id: row_18 };
            let row_ids_20: Vec<usize> = vec![row_18];
            // Select operator consume
            if view_19.get(col_29).unwrap() >= ValueRef::Integer(2000) {
                // Select operator consume
                if view_19.get(col_29).unwrap() <= ValueRef::Integer(2003) {
                    builder_14.insert(vec![view_19.get(col_23).unwrap()], row_ids_20);
                }
            }
        }
    }
    );
    let map_15 = builder_14.finalize();
    // Probe Phase
    // TableScan
    (0..table_33.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | rows_34 | {
        for row_35 in rows_34  {
            let view_36 = RowView{ table: &table_33, row_id: row_35 };
            let row_ids_37: Vec<usize> = vec![row_35];
            // Select operator consume
            if view_36.get(col_40).unwrap() >= ValueRef::Integer(1) {
                // Select operator consume
                if view_36.get(col_40).unwrap() <= ValueRef::Integer(350000) {
                    if let Some(probe_vector_43) = map_15.get(&vec![view_36.get(col_41).unwrap()]) {
                        for probe_44 in probe_vector_43 {
                            let row_ids_45: Vec<_> = { let mut tmp: Vec<usize> = probe_44.clone(); tmp.extend(row_ids_37.iter().copied()); tmp };
                            builder_12.insert(vec![RowView{ table: &table_16, row_id: *row_ids_45.get(0).unwrap() }.get(col_23).unwrap()], row_ids_45);
                        }
                    }
                }
            }
        }
    }
    );
    let map_13 = builder_12.finalize();
    // Probe Phase
    // TableScan
    (0..table_46.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | rows_47 | {
        for row_48 in rows_47  {
            let view_49 = RowView{ table: &table_46, row_id: row_48 };
            let row_ids_50: Vec<usize> = vec![row_48];
            // Select operator consume
            if view_49.get(col_53).unwrap() >= ValueRef::Integer(1) {
                // Select operator consume
                if view_49.get(col_53).unwrap() <= ValueRef::Integer(350000) {
                    if let Some(probe_vector_63) = map_13.get(&vec![view_49.get(col_57).unwrap()]) {
                        for probe_64 in probe_vector_63 {
                            let row_ids_65: Vec<_> = { let mut tmp: Vec<usize> = probe_64.clone(); tmp.extend(row_ids_50.iter().copied()); tmp };
                            builder_10.insert(vec![RowView{ table: &table_16, row_id: *row_ids_65.get(0).unwrap() }.get(col_23).unwrap()], row_ids_65);
                        }
                    }
                }
            }
        }
    }
    );
    let map_11 = builder_10.finalize();
    // Probe Phase
    // TableScan
    (0..table_66.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | rows_67 | {
        for row_68 in rows_67  {
            let view_69 = RowView{ table: &table_66, row_id: row_68 };
            let row_ids_70: Vec<usize> = vec![row_68];
            // Select operator consume
            if view_69.get(col_71).unwrap() >= ValueRef::Integer(1) {
                // Select operator consume
                if view_69.get(col_71).unwrap() <= ValueRef::Integer(350000) {
                    if let Some(probe_vector_76) = map_11.get(&vec![view_69.get(col_74).unwrap()]) {
                        for probe_77 in probe_vector_76 {
                            let row_ids_78: Vec<_> = { let mut tmp: Vec<usize> = probe_77.clone(); tmp.extend(row_ids_70.iter().copied()); tmp };
                            builder_8.insert(vec![RowView{ table: &table_66, row_id: *row_ids_78.get(3).unwrap() }.get(col_73).unwrap()], row_ids_78);
                        }
                    }
                }
            }
        }
    }
    );
    let map_9 = builder_8.finalize();
    // Probe Phase
    // TableScan
    (0..table_79.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | rows_80 | {
        for row_81 in rows_80  {
            let view_82 = RowView{ table: &table_79, row_id: row_81 };
            let row_ids_83: Vec<usize> = vec![row_81];
            if let Some(probe_vector_86) = map_9.get(&vec![view_82.get(col_84).unwrap()]) {
                for probe_87 in probe_vector_86 {
                    let row_ids_88: Vec<_> = { let mut tmp: Vec<usize> = probe_87.clone(); tmp.extend(row_ids_83.iter().copied()); tmp };
                    builder_6.insert(vec![RowView{ table: &table_16, row_id: *row_ids_88.get(0).unwrap() }.get(col_23).unwrap()], row_ids_88);
                }
            }
        }
    }
    );
    let map_7 = builder_6.finalize();
    // Probe Phase
    // TableScan
    (0..table_89.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | rows_90 | {
        for row_91 in rows_90  {
            let view_92 = RowView{ table: &table_89, row_id: row_91 };
            let row_ids_93: Vec<usize> = vec![row_91];
            // Select operator consume
            if view_92.get(col_94).unwrap() >= ValueRef::Integer(1) {
                // Select operator consume
                if view_92.get(col_94).unwrap() <= ValueRef::Integer(350000) {
                    if let Some(probe_vector_97) = map_7.get(&vec![view_92.get(col_96).unwrap()]) {
                        for probe_98 in probe_vector_97 {
                            let row_ids_99: Vec<_> = { let mut tmp: Vec<usize> = probe_98.clone(); tmp.extend(row_ids_93.iter().copied()); tmp };
                            builder_4.insert(vec![RowView{ table: &table_89, row_id: *row_ids_99.get(5).unwrap() }.get(col_95).unwrap()], row_ids_99);
                        }
                    }
                }
            }
        }
    }
    );
    let map_5 = builder_4.finalize();
    // Probe Phase
    // TableScan
    (0..table_100.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | rows_101 | {
        for row_102 in rows_101  {
            let view_103 = RowView{ table: &table_100, row_id: row_102 };
            let row_ids_104: Vec<usize> = vec![row_102];
            if let Some(probe_vector_108) = map_5.get(&vec![view_103.get(col_105).unwrap()]) {
                for probe_109 in probe_vector_108 {
                    let row_ids_110: Vec<_> = { let mut tmp: Vec<usize> = probe_109.clone(); tmp.extend(row_ids_104.iter().copied()); tmp };
                    builder_2.insert(vec![RowView{ table: &table_33, row_id: *row_ids_110.get(1).unwrap() }.get(col_38).unwrap()], row_ids_110);
                }
            }
        }
    }
    );
    let map_3 = builder_2.finalize();
    // Probe Phase
    // TableScan
    (0..table_111.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | rows_112 | {
        for row_113 in rows_112  {
            let view_114 = RowView{ table: &table_111, row_id: row_113 };
            let row_ids_115: Vec<usize> = vec![row_113];
            if let Some(probe_vector_123) = map_3.get(&vec![view_114.get(col_117).unwrap()]) {
                for probe_124 in probe_vector_123 {
                    let row_ids_125: Vec<_> = { let mut tmp: Vec<usize> = probe_124.clone(); tmp.extend(row_ids_115.iter().copied()); tmp };
                    builder_0.insert(vec![RowView{ table: &table_33, row_id: *row_ids_125.get(1).unwrap() }.get(col_39).unwrap()], row_ids_125);
                }
            }
        }
    }
    );
    let map_1 = builder_0.finalize();
    // Probe Phase
    // TableScan
    (0..table_126.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | rows_127 | {
        for row_128 in rows_127  {
            let view_129 = RowView{ table: &table_126, row_id: row_128 };
            let row_ids_130: Vec<usize> = vec![row_128];
            if let Some(probe_vector_133) = map_1.get(&vec![view_129.get(col_131).unwrap()]) {
                for probe_134 in probe_vector_133 {
                    let row_ids_135: Vec<_> = { let mut tmp: Vec<usize> = probe_134.clone(); tmp.extend(row_ids_130.iter().copied()); tmp };
                    out.inc_count();
                }
            }
        }
    }
    );
    let (text, rows) = out.finalize();
    print!("{}", text);
    println!("Total rows is : {}", rows);
    Ok(out_count)
}

// --------------------------------
// Total Building time : 29.846792ms
// Time taken to build and insert into PHF : 17.819666ms
// Time taken to deduplicate the keys : 10.555ms
// Total Building time : 2.598292ms
// Time taken to build and insert into PHF : 1.722042ms
// Time taken to deduplicate the keys : 832.042µs
// Total Building time : 571.584µs
// Time taken to build and insert into PHF : 343.167µs
// Time taken to deduplicate the keys : 195.875µs
// Total Building time : 8.43575ms
// Time taken to build and insert into PHF : 3.758792ms
// Time taken to deduplicate the keys : 4.11075ms
// Total Building time : 12.8185ms
// Time taken to build and insert into PHF : 4.29775ms
// Time taken to deduplicate the keys : 4.545708ms
// Total Building time : 786.836958ms
// Time taken to build and insert into PHF : 244.327125ms
// Time taken to deduplicate the keys : 333.902209ms
// Total Building time : 916.470417ms
// Time taken to build and insert into PHF : 291.4145ms
// Time taken to deduplicate the keys : 313.563416ms
// Total Building time : 747.111292ms
// Time taken to build and insert into PHF : 263.659417ms
// Time taken to deduplicate the keys : 289.510959ms
//
// Total rows is : 12330283
// Compilation time is : 1.533941625s
// Execution time is : 10.372095542s