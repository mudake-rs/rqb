use rqb::dsl::fts::{plainto_tsquery, ts_rank};
use rqb::dsl::param;
use rqb::prelude::*;
use serde_json::Value;
use uuid::Uuid;

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
    let query = plainto_tsquery(query_text);
    let full_text = select(vector_documents::table())
        // `.item(...)` starts an explicit projection; default fields are not
        // selected unless they are added with `.column(...)`.
        .column(vector_documents::ID)
        .item(ts_rank(search_index.clone(), query.clone()).alias("rank"))
        .filter(search_index.predicate("@@", query.clone()))
        .order_desc(ts_rank(vector_documents::SEARCH_INDEX_META.expr(), query))
        .build()?;

    assert_eq!(
        full_text.sql,
        "SELECT \"id\", ts_rank(\"search_index\", plainto_tsquery($1)) AS \"rank\" FROM \"sample\".\"vector_documents\" WHERE \"search_index\" @@ plainto_tsquery($2) ORDER BY ts_rank(\"search_index\", plainto_tsquery($3)) DESC"
    );
    assert_eq!(full_text.params.len(), 3);

    println!("{}", default_projection.sql);
    println!("{}", vector_search.sql);
    println!("{}", full_text.sql);
    Ok(())
}
