use rqb::WriteRecord;

#[derive(WriteRecord)]
struct MissingFields {
    id: String,
}

fn main() {}
