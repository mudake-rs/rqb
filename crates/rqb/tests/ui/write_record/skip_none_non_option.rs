use rqb::{Field, FieldType, WriteRecord};

mod users {
    use super::*;

    pub const EMAIL: Field = Field::new("email", FieldType::Text);
}

#[derive(WriteRecord)]
#[rqb(fields = users)]
struct BadSkipNone {
    #[rqb(skip_none)]
    email: String,
}

fn main() {}
