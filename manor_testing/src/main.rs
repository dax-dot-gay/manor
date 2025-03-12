use std::error::Error;

use manor::{Model, Schema as _, bson::oid::ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Model, Clone, Debug)]
#[manor(schema(id = "id"))]
pub struct Testing {
    #[serde(rename = "_id")]
    id: ObjectId,
    test: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("{}, {}", Testing::collection_name(), Testing::id_field());
    Ok(())
}
