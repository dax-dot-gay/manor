use manor::{schema, Client, Collection, Link, Model, bson::Uuid};

#[schema(collection = "sessions")]
pub struct Session {
    #[field(id = Uuid::new)]
    pub id: Uuid,

    #[serde(default)]
    pub user: Option<Link<User>>,
}

#[schema(collection = "users")]
pub struct User {
    #[field(id = Uuid::new)]
    pub id: Uuid,
    pub username: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Client::connect_with_uri("mongodb://manor:manor@mongodb:27017/", "manor-testing").await?.as_global();

    let sess = Session {id: Uuid::new(), user: None, _collection: None};
    println!("{sess:?}");
    sess.save().await?;

    let found = Collection::<Session>::new().get(sess.id()).await;
    println!("{found:?}");

    sess.save().await?;

    Ok(())
}
