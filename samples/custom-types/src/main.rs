use rqb::prelude::*;
use rqb_sample_base::{
    ADA_USER_ID,
    schema::{types, withdrawals},
};
use serde::Deserialize;
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

#[derive(Debug, Deserialize)]
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

    // 1. CLI generation exposes the domain as TypeSpec metadata.
    println!(
        "generated custom type: {} ({:?})",
        types::UINT_256.name,
        types::UINT_256.value_repr
    );

    // 2. Writes bind exact domain values as decimal strings.
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

    // 3. Reads select the same domain back as strings, including arrays.
    let rows = select(withdrawals::dataset())
        .fields([
            withdrawals::ID.into(),
            withdrawals::AMOUNT.into(),
            withdrawals::AMOUNT_HISTORY.alias("amount_history"),
            withdrawals::WALLET_ADDRESS.alias("wallet_address"),
        ])
        .filter(withdrawals::AMOUNT.gte("9007199254740993"))
        .fetch_all_as::<Withdrawal>(&db)
        .await?;
    for row in &rows {
        println!(
            "withdrawal {} amount={} history={:?} wallet={}",
            row.id, row.amount, row.amount_history, row.wallet_address
        );
    }

    // 4. Delete the row created by this sample.
    delete(withdrawals::dataset())
        .filter(withdrawals::ID.eq(id))
        .execute(&db)
        .await?;

    Ok(())
}
