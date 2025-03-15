use proc_macro::TokenStream;

mod schema;
mod util;

/// This macro generates a filled-out Model from a template struct, as well as an associated Builder.
/// The attribute itself follows this syntax:
/// 
/// ```
/// #[schema(collection = "optional collection name", schema_name = OptionalSchemaName, builder_name = OptionalBuilderName)]
/// ```
/// 
/// Individual fields may also be marked with the `#[field(...)` attribute.
/// 
/// At most one field may be marked with `#[field(id = <generator>)]`. This will mark this field as the model's ID field, and use the passed generator to generate IDs. 
/// The generator can be a closure with no arguments, a function call, or a path to a function to call (also with no arguments)
/// Any other `field(...)` parameters will be ignored on the ID field. If an ID field is not specified, the macro will default to `id: bson::oid::ObjectId`.
/// 
/// Non-ID fields can be marked with `#[field(alias = "some string")]`. This is a simplified equivalent of `#[serde(rename = "value")]`.
/// 
/// ---
/// 
/// An example schema:
/// ```
/// use manor::schema;
/// use uuid::Uuid;
/// 
/// #[schema(collection = "users")]
/// pub struct User {
///     #[field(id = Uuid::new_v4)]
///     pub user_id: Uuid,
///     
///     #[field(alias = "username")]
///     pub name: String,
///     pub password: String
/// }
/// ```
#[proc_macro_attribute]
pub fn schema(args: TokenStream, input: TokenStream) -> TokenStream {
    schema::generate_schema(args, input)
}