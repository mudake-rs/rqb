//! Use a project-specific Postgres domain through TypeSpec.
//!
//! `uint_256` is represented as a decimal string so large values never pass
//! through `f64`. Selected values are cast to text and deserialize as `String`.

use rqb::prelude::*;
use serde::{Deserialize, Serialize};

const UINT_256: TypeSpec = TypeSpec::domain(Some("public"), "uint_256")
    .base(TypeFamily::Numeric)
    .value_repr(ValueRepr::DecimalString)
    .select_repr(SelectRepr::Text);

const ID: Field = Field::new("id", FieldType::Uuid);
const USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
const AMOUNT: Field = Field::new("amount", FieldType::Custom(&UINT_256));
const AMOUNT_HISTORY: Field = Field::mapped(
    "amountHistory",
    "amount_history",
    FieldType::Array(ElemType::Custom(&UINT_256)),
)
.sortable(false);
const WALLET_ADDRESS: Field = Field::mapped("walletAddress", "wallet_address", FieldType::Text);

fn withdrawals() -> Dataset {
    Dataset::table("withdrawals").fields([ID, USER_ID, AMOUNT, AMOUNT_HISTORY, WALLET_ADDRESS])
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewWithdrawal {
    id: String,
    user_id: String,
    amount: String,
    amount_history: Vec<String>,
    wallet_address: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct Withdrawal {
    id: String,
    user_id: String,
    amount: String,
    amount_history: Vec<String>,
    wallet_address: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let row = NewWithdrawal {
        id: "60000000-0000-0000-0000-000000009999".to_owned(),
        user_id: "10000000-0000-0000-0000-000000000001".to_owned(),
        amount: "900719925474099312345678901234567890".to_owned(),
        amount_history: vec![
            "1".to_owned(),
            "900719925474099312345678901234567890".to_owned(),
        ],
        wallet_address: "0xabc".to_owned(),
    };

    let insert_sql = insert(withdrawals())
        .value(&row)
        .returning_all()
        .build_pg()?;
    println!("-- insert exact domain");
    println!("{}", insert_sql.debug_sql());

    let select_sql = select(withdrawals())
        .filter(AMOUNT.gt("9007199254740993"))
        .build_pg()?;
    let _serde_shape = std::any::type_name::<Withdrawal>();
    println!("-- select exact domain");
    println!("{}", select_sql.debug_sql());

    Ok(())
}
