use rqb::prelude::*;
use rqb_sample_base::{
    ADA_USER_ID,
    schema::{types, withdrawals},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewWithdrawal {
    id: Uuid,
    user_id: Uuid,
    amount: String,
    amount_history: Vec<String>,
    wallet_address: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Withdrawal {
    id: Uuid,
    amount: String,
    amount_history: Vec<String>,
    wallet_address: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;
    let id = Uuid::new_v4();
    let huge = "900719925474099312345678901234567890".to_owned();

    println!(
        "generated custom type: {} ({:?})",
        types::UINT_256.name,
        types::UINT_256.value_repr
    );

    insert(withdrawals::dataset())
        .value(&NewWithdrawal {
            id,
            user_id: rqb_sample_base::uuid(ADA_USER_ID),
            amount: huge.clone(),
            amount_history: vec!["1".to_owned(), huge.clone()],
            wallet_address: "0xsample".to_owned(),
        })
        .execute(&db)
        .await?;

    let rows = select(withdrawals::dataset())
        .fields([
            withdrawals::ID,
            withdrawals::AMOUNT,
            withdrawals::AMOUNT_HISTORY,
            withdrawals::WALLET_ADDRESS,
        ])
        .filter(withdrawals::AMOUNT.gte("9007199254740993"))
        .fetch_as::<Withdrawal>(&db)
        .await?;
    for row in &rows {
        println!(
            "withdrawal {} amount={} history={:?} wallet={}",
            row.id, row.amount, row.amount_history, row.wallet_address
        );
    }

    delete(withdrawals::dataset())
        .filter(withdrawals::ID.eq(id))
        .execute(&db)
        .await?;

    Ok(())
}
