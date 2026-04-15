use rqb::prelude::*;
use rqb_sample_base::{
    ADA_USER_ID,
    schema::{types, withdrawals},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, WriteRecord)]
#[rqb(fields = withdrawals)]
struct NewWithdrawal {
    id: Uuid,
    user_id: Uuid,
    amount: String,
    amount_history: Vec<String>,
    wallet_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WithdrawalRow {
    id: Uuid,
    amount: String,
    amount_history: Vec<String>,
    wallet_address: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;
    let id = Uuid::new_v4();
    let huge = "900719925474099312345678901234567891".to_owned();

    println!(
        "generated type: {} value={:?} select={:?}",
        types::UINT_256.name,
        types::UINT_256.value_repr,
        types::UINT_256.select_repr
    );

    insert(withdrawals::dataset())
        .value(&NewWithdrawal {
            id,
            user_id: rqb_sample_base::uuid(ADA_USER_ID),
            amount: huge.clone(),
            amount_history: vec!["1".to_owned(), huge.clone()],
            wallet_address: "0xnumeric".to_owned(),
        })
        .execute(&db)
        .await?;

    let rows = select(withdrawals::dataset())
        .fields([
            withdrawals::ID.into(),
            withdrawals::AMOUNT.into(),
            withdrawals::AMOUNT_HISTORY.alias("amount_history"),
            withdrawals::WALLET_ADDRESS.alias("wallet_address"),
        ])
        .filter(withdrawals::AMOUNT.gt("9007199254740993"))
        .order_by(withdrawals::AMOUNT.desc())
        .fetch_all_as::<WithdrawalRow>(&db)
        .await?;

    println!("{}", serde_json::to_string_pretty(&rows)?);

    delete(withdrawals::dataset())
        .filter(withdrawals::ID.eq(id))
        .execute(&db)
        .await?;

    Ok(())
}
