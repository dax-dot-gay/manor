use manor::{Link, schema};
use uuid::Uuid;

#[schema(collection = "sessions")]
pub struct Session {
    #[field(id = Uuid::new_v4)]
    pub id: Uuid,

    #[serde(default)]
    pub user: Option<Link<User>>,
}

#[schema(collection = "users")]
pub struct User {
    #[field(id = Uuid::new_v4)]
    pub id: Uuid,
    pub username: String,
}

fn main() {
    ()
}
