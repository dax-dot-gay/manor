use proc_macro::TokenStream;

mod schema;
mod util;
mod actions;

#[proc_macro_attribute]
pub fn schema(args: TokenStream, input: TokenStream) -> TokenStream {
    schema::generate_schema(args, input)
}