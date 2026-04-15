use rqb::prelude::*;
use uuid::Uuid;

use crate::schema::orders;
use crate::services::users;
use crate::types::{CheckoutResponse, CreateOrder, OrderRow};

pub async fn checkout(pool: &PgPool, input: CreateOrder) -> rqb::Result<CheckoutResponse> {
    tx!(pool, |conn| {
        let user = users::find(&mut *conn, input.user_id).await?;
        let order = create(&mut *conn, input).await?;
        Ok(CheckoutResponse { user, order })
    })
    .await
}

async fn create(conn: &mut PgConnection, input: CreateOrder) -> rqb::Result<OrderRow> {
    insert(orders::table())
        .set(orders::ID.set(Uuid::new_v4()))
        .values(&input)
        .set(orders::STATUS.set("open"))
        .returning_all()
        .fetch_one_as::<OrderRow>(conn)
        .await
}

pub async fn cancel_open_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> rqb::Result<()> {
    update(orders::table())
        .set(orders::STATUS.set("canceled"))
        .filter(orders::USER_ID.eq(user_id))
        .filter(orders::STATUS.eq("open"))
        .execute(conn)
        .await?;
    Ok(())
}
