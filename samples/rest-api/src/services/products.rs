use rqb::prelude::*;
use rqb_sample_schema::products;
use uuid::Uuid;

use crate::types::{ProductRow, UpsertProduct};

pub async fn upsert(db: &PgPool, input: UpsertProduct) -> rqb::Result<ProductRow> {
    // Catalog sync endpoints usually want "insert new SKU, otherwise refresh
    // mutable product data". `do_update_excluded((...))` keeps that common
    // Postgres pattern from becoming four repeated `field.set_excluded()` calls.
    insert(products::table())
        .set(products::ID.set(Uuid::new_v4()))
        .values(&input)
        .on_conflict(products::SKU)
        .do_update_excluded((
            products::NAME,
            products::PRICE_CENTS,
            products::ATTRIBUTES,
            products::TAGS,
        ))
        .returning_all()
        .fetch_one_as::<ProductRow>(db)
        .await
}
