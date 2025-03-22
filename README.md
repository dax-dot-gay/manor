# `manor` - MongoDB for Rust, Simplified

![docs.rs](https://img.shields.io/docsrs/manor) ![Crates.io License](https://img.shields.io/crates/l/manor)



A highly-automated MongoDB ORM, extending on the `mongodb` crate to bring tighter GridFS integration, cross-document linking, and extremely low-code schema specification.

> Note: Currently not tested exhaustively, and likely not ready for production. Still under active development.

### Installation

```bash
cargo add manor
```

### Example Usage

A (very) brief example of this crate's usage is as follows:

```rust
use manor::{schema, Client, Collection};
use uuid::Uuid;

// Set up a schema, defines [User] and [UserBuilder]
#[schema(collection = "users")]
pub struct User {
    #[field(id = Uuid::new_v4)]
    pub id: Uuid,

    pub username: String,
    pub password: String
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a client and assign it to the global instance
    Client::connect_with_uri("mongodb://...", "my_app")?.as_global();

    let user = UserBuilder::default().username("alice").password("bob").build()?;
    user.save().await?;

    let users = Collection::<User>::new();
    for user in users.find_many(bson::doc! {}).await {
        println!("{user:?}");
    }

    Ok(())
}

```