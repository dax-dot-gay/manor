use manor::schema;

#[schema]
struct Testing {
    beans: String
}

impl Testing {
    pub fn test(&self) {
        println!("{}", self.beans);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{:?}", TestingBuilder::create_empty().beans("Test").build()?);
    Ok(())
}
