use rqb::dsl::{param, phraseto_tsquery, ts_rank};
use rqb::prelude::*;
use serde_json::Value;
use uuid::Uuid;

mod mapped_schema;
mod types;

rqb::schema! {
    table sample.vector_documents {
        id: uuid = Uuid,
        // Unknown or extension types can stay raw-only metadata. They are still
        // projected by default but do not get a typed Field<T> constant.
        status: document_state,
        embedding: vector,
        search_index: tsvector,
        metadata: jsonb = Value,
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mapped = insert(mapped_schema::wallets::table())
        .set(mapped_schema::wallets::ID.set(1))
        .set(mapped_schema::wallets::BALANCE.set(types::Cents(1250)))
        .build()?;
    assert_eq!(mapped.params.len(), 2);
    // Even without typed extension support, generated metadata is enough for a
    // useful default projection.
    let default_projection = select(vector_documents::table()).build()?;
    assert_eq!(
        default_projection.sql,
        "SELECT \"id\", \"status\", \"embedding\", \"search_index\", \"metadata\" FROM \"sample\".\"vector_documents\""
    );

    assert_eq!(vector_documents::STATUS_META.db, "status");
    assert_eq!(vector_documents::EMBEDDING_META.pg, "vector");
    assert_eq!(vector_documents::SEARCH_INDEX_META.pg, "tsvector");

    // Raw-only extension columns can still participate in server-owned custom
    // operators when the SQL shape is known by the application.
    let embedding = vector_documents::EMBEDDING_META.expr();
    let probe = param("[0.1,0.2,0.3]".to_owned()).cast("vector");
    let vector_search = select(vector_documents::table())
        .filter(embedding.op("<->", probe).lt(0.5_f64))
        .build()?;

    assert_eq!(
        vector_search.sql,
        "SELECT \"id\", \"status\", \"embedding\", \"search_index\", \"metadata\" FROM \"sample\".\"vector_documents\" WHERE (\"embedding\" <-> CAST($1 AS vector)) < $2"
    );
    assert_eq!(vector_search.params.len(), 2);
    assert!(vector_search.cacheable);

    let query_text = "rust postgres";
    let search_index = vector_documents::SEARCH_INDEX_META.expr();
    let ts_query = phraseto_tsquery(query_text);
    let full_text = select(vector_documents::table())
        // `default_columns()` expands the same schema metadata that the default
        // projection would render, then computed items can be appended.
        .default_columns()
        .expr_as(ts_rank(search_index.clone(), ts_query.clone()), "rank")
        .filter(search_index.predicate("@@", ts_query.clone()))
        .order_desc(ts_rank(vector_documents::SEARCH_INDEX_META, ts_query))
        .build()?;

    assert_eq!(
        full_text.sql,
        "SELECT \"id\", \"status\", \"embedding\", \"search_index\", \"metadata\", ts_rank(\"search_index\", phraseto_tsquery($1)) AS \"rank\" FROM \"sample\".\"vector_documents\" WHERE \"search_index\" @@ phraseto_tsquery($2) ORDER BY ts_rank(\"search_index\", phraseto_tsquery($3)) DESC"
    );
    assert_eq!(full_text.params.len(), 3);

    println!("{}", default_projection.sql);
    println!("{}", vector_search.sql);
    println!("{}", full_text.sql);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires schema.sql and RQB_TEST_DATABASE_URL"]
    async fn configured_domain_codec_binds_decodes_and_searches() {
        use mapped_schema::wallets as w;
        let pool = sqlx::PgPool::connect(&std::env::var("RQB_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let mut tx = pool.begin().await.unwrap();
        let value = insert(w::table())
            .set(w::ID.set(1))
            .set(w::BALANCE.set(types::Cents(1250)))
            .returning(w::BALANCE)
            .fetch_one_scalar::<types::Cents>(&mut *tx)
            .await
            .unwrap();
        assert_eq!(value, types::Cents(1250));
        let filter = serde_json::from_value(
            serde_json::json!({"field":"balance","operator":"equals","value":1250}),
        )
        .unwrap();
        let value = select(w::table())
            .column(w::BALANCE)
            .apply_filter(filter)
            .unwrap()
            .fetch_one_scalar::<types::Cents>(&mut *tx)
            .await
            .unwrap();
        assert_eq!(value, types::Cents(1250));
        let err = update(w::table())
            .set(w::BALANCE.set(types::Cents(-1)))
            .filter(w::ID.eq(1))
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert!(matches!(err, rqb::Error::CheckViolation(_)));
        tx.rollback().await.unwrap();
    }
}
