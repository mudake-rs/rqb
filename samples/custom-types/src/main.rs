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
        metadata: jsonb = Value,
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let default_projection = select(vector_documents::table()).build()?;
    assert_eq!(
        default_projection.sql,
        "SELECT \"id\", \"status\", \"embedding\", \"metadata\" FROM \"sample\".\"vector_documents\""
    );

    assert_eq!(vector_documents::STATUS_META.db, "status");
    assert_eq!(vector_documents::EMBEDDING_META.pg, "vector");

    // Raw-only extension columns can still participate in server-owned custom
    // operators when the SQL shape is known by the application.
    let embedding = vector_documents::EMBEDDING_META.expr();
    let probe = param("[0.1,0.2,0.3]".to_owned()).cast("vector");
    let vector_search = select(vector_documents::table())
        .filter(embedding.op("<->", probe).lt(0.5_f64))
        .build()?;

    assert_eq!(
        vector_search.sql,
        "SELECT \"id\", \"status\", \"embedding\", \"metadata\" FROM \"sample\".\"vector_documents\" WHERE (\"embedding\" <-> CAST($1 AS vector)) < $2"
    );
    assert_eq!(vector_search.params.len(), 2);
    assert!(vector_search.cacheable);

    println!("{}", default_projection.sql);
    println!("{}", vector_search.sql);
    Ok(())
}
