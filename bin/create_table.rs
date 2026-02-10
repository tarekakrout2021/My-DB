use std::error::Error;

use imlab::db::Database;
use imlab::parser::ast::{Column as ASTColumn, Table as ASTTable};
use imlab::types::Type;

fn integer() -> Type {
    Type::new_integer()
}
fn numeric(p: u32, s: u16) -> Type {
    Type::new_numeric(p, s)
}
fn text() -> Type {
    Type::new_text()
}
fn timestamp() -> Type {
    Type::new_timestamp()
}

pub fn tpcc_tables() -> Vec<ASTTable> {
    vec![
        warehouse(),
        district(),
        customer(),
        history(),
        neworder(),
        order_tbl(),
        orderline(),
        item(),
        stock(),
    ]
}

pub fn create_tpcc_tables(db: &mut Database) -> Result<(), Box<dyn Error>> {
    for t in tpcc_tables() {
        db.create_table(&t)?;
    }
    Ok(())
}

fn warehouse() -> ASTTable {
    let cols = vec![
        ASTColumn {
            id: "w_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "w_name".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "w_street_1".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "w_street_2".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "w_city".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "w_state".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "w_zip".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "w_tax".into(),
            r#type: numeric(4, 4),
            not_null: true,
        },
        ASTColumn {
            id: "w_ytd".into(),
            r#type: numeric(12, 2),
            not_null: true,
        },
    ];
    ASTTable {
        id: "warehouse".into(),
        columns: cols,
        primary_key: vec!["w_id".into()],
    }
}

fn district() -> ASTTable {
    let cols = vec![
        ASTColumn {
            id: "d_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "d_w_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "d_name".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "d_street_1".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "d_street_2".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "d_city".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "d_state".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "d_zip".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "d_tax".into(),
            r#type: numeric(4, 4),
            not_null: true,
        },
        ASTColumn {
            id: "d_ytd".into(),
            r#type: numeric(12, 2),
            not_null: true,
        },
        ASTColumn {
            id: "d_next_o_id".into(),
            r#type: integer(),
            not_null: true,
        },
    ];
    ASTTable {
        id: "district".into(),
        columns: cols,
        primary_key: vec!["d_w_id".into(), "d_id".into()],
    }
}

fn customer() -> ASTTable {
    let cols = vec![
        ASTColumn {
            id: "c_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "c_d_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "c_w_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "c_first".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_middle".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_last".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_street_1".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_street_2".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_city".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_state".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_zip".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_phone".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_since".into(),
            r#type: timestamp(),
            not_null: true,
        },
        ASTColumn {
            id: "c_credit".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "c_credit_lim".into(),
            r#type: numeric(12, 2),
            not_null: true,
        },
        ASTColumn {
            id: "c_discount".into(),
            r#type: numeric(4, 4),
            not_null: true,
        },
        ASTColumn {
            id: "c_balance".into(),
            r#type: numeric(12, 2),
            not_null: true,
        },
        ASTColumn {
            id: "c_ytd_paymenr".into(),
            r#type: numeric(12, 2),
            not_null: true,
        },
        ASTColumn {
            id: "c_payment_cnt".into(),
            r#type: numeric(4, 0),
            not_null: true,
        },
        ASTColumn {
            id: "c_delivery_cnt".into(),
            r#type: numeric(4, 0),
            not_null: true,
        },
        ASTColumn {
            id: "c_data".into(),
            r#type: text(),
            not_null: true,
        },
    ];
    ASTTable {
        id: "customer".into(),
        columns: cols,
        primary_key: vec!["c_w_id".into(), "c_d_id".into(), "c_id".into()],
    }
}

fn history() -> ASTTable {
    let cols = vec![
        ASTColumn {
            id: "h_c_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "h_c_d_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "h_c_w_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "h_d_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "h_w_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "h_date".into(),
            r#type: timestamp(),
            not_null: true,
        },
        ASTColumn {
            id: "h_amount".into(),
            r#type: numeric(6, 2),
            not_null: true,
        },
        ASTColumn {
            id: "h_data".into(),
            r#type: text(),
            not_null: true,
        },
    ];
    ASTTable {
        id: "history".into(),
        columns: cols,
        primary_key: vec![], // no PK in the DDL
    }
}

fn neworder() -> ASTTable {
    let cols = vec![
        ASTColumn {
            id: "no_o_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "no_d_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "no_w_id".into(),
            r#type: integer(),
            not_null: true,
        },
    ];
    ASTTable {
        id: "neworder".into(),
        columns: cols,
        primary_key: vec!["no_w_id".into(), "no_d_id".into(), "no_o_id".into()],
    }
}

fn order_tbl() -> ASTTable {
    let cols = vec![
        ASTColumn {
            id: "o_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "o_d_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "o_w_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "o_c_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "o_entry_d".into(),
            r#type: timestamp(),
            not_null: true,
        },
        ASTColumn {
            id: "o_carrier_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "o_ol_cnt".into(),
            r#type: numeric(2, 0),
            not_null: true,
        },
        ASTColumn {
            id: "o_all_local".into(),
            r#type: numeric(1, 0),
            not_null: true,
        },
    ];
    ASTTable {
        id: "order".into(), // string literal; not a Rust identifier
        columns: cols,
        primary_key: vec!["o_w_id".into(), "o_d_id".into(), "o_id".into()],
    }
}

fn orderline() -> ASTTable {
    let cols = vec![
        ASTColumn {
            id: "ol_o_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "ol_d_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "ol_w_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "ol_number".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "ol_i_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "ol_supply_w_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "ol_delivery_d".into(),
            r#type: timestamp(),
            not_null: true,
        },
        ASTColumn {
            id: "ol_quantity".into(),
            r#type: numeric(2, 0),
            not_null: true,
        },
        ASTColumn {
            id: "ol_amount".into(),
            r#type: numeric(6, 2),
            not_null: true,
        },
        ASTColumn {
            id: "ol_dist_info".into(),
            r#type: text(),
            not_null: true,
        },
    ];
    ASTTable {
        id: "orderline".into(),
        columns: cols,
        primary_key: vec![
            "ol_w_id".into(),
            "ol_d_id".into(),
            "ol_o_id".into(),
            "ol_number".into(),
        ],
    }
}

fn item() -> ASTTable {
    let cols = vec![
        ASTColumn {
            id: "i_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "i_im_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "i_name".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "i_price".into(),
            r#type: numeric(5, 2),
            not_null: true,
        },
        ASTColumn {
            id: "i_data".into(),
            r#type: text(),
            not_null: true,
        },
    ];
    ASTTable {
        id: "item".into(),
        columns: cols,
        primary_key: vec!["i_id".into()],
    }
}

fn stock() -> ASTTable {
    let cols = vec![
        ASTColumn {
            id: "s_i_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "s_w_id".into(),
            r#type: integer(),
            not_null: true,
        },
        ASTColumn {
            id: "s_quantity".into(),
            r#type: numeric(4, 0),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_01".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_02".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_03".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_04".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_05".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_06".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_07".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_08".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_09".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_dist_10".into(),
            r#type: text(),
            not_null: true,
        },
        ASTColumn {
            id: "s_ytd".into(),
            r#type: numeric(8, 0),
            not_null: true,
        },
        ASTColumn {
            id: "s_order_cnt".into(),
            r#type: numeric(4, 0),
            not_null: true,
        },
        ASTColumn {
            id: "s_remote_cnt".into(),
            r#type: numeric(4, 0),
            not_null: true,
        },
        ASTColumn {
            id: "s_data".into(),
            r#type: text(),
            not_null: true,
        },
    ];
    ASTTable {
        id: "stock".into(),
        columns: cols,
        primary_key: vec!["s_w_id".into(), "s_i_id".into()],
    }
}
