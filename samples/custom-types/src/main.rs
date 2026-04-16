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

    // Server-owned raw predicates are the escape hatch for extension operators.
    let vector_search = select(vector_documents::table())
        .filter(BoolExpr::Raw {
            sql: "embedding <-> ?::vector < ?".to_owned(),
            params: vec![
                Param::typed("[0.1,0.2,0.3]".to_owned()),
                Param::typed(0.5_f64),
            ],
        })
        .build()?;

    assert_eq!(
        vector_search.sql,
        "SELECT \"id\", \"status\", \"embedding\", \"metadata\" FROM \"sample\".\"vector_documents\" WHERE embedding <-> $1::vector < $2"
    );
    assert_eq!(vector_search.params.len(), 2);
    assert!(!vector_search.cacheable);

    println!("{}", default_projection.sql);
    println!("{}", vector_search.sql);
    Ok(())
}
