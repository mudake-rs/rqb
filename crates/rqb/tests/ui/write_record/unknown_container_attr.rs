use rqb::{Field, FieldType, WriteRecord};

mod users {
    use super::*;

    pub const EMAIL: Field = Field::new("email", FieldType::Text);
}

#[derive(WriteRecord)]
#[rqb(fields = users, unknown)]
struct UnknownContainerAttr {
    email: String,
}

fn main() {}
