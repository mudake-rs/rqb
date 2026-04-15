use rqb::{Field, FieldType, WriteRecord};

mod users {
    use super::*;

    pub const EMAIL: Field = Field::new("email", FieldType::Text);
}

#[derive(WriteRecord)]
#[rqb(fields = users)]
struct UnknownFieldAttr {
    #[rqb(unknown)]
    email: String,
}

fn main() {}
