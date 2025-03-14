use manor::schema;

#[schema]
struct Testing {
    #[field(id = manor::uuid::Uuid::new_v4)]
    id: manor::uuid::Uuid
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{:?}", TestingBuilder::create_empty().build()?);
    Ok(())
}
