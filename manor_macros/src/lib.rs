use proc_macro::TokenStream;

mod schema;
mod util;

#[proc_macro_attribute]
pub fn schema(args: TokenStream, input: TokenStream) -> TokenStream {
    schema::generate_schema(args, input)
}

#[proc_macro_attribute]
pub fn field(args: TokenStream, input: TokenStream) -> TokenStream {
    println!("{} @ {}", args.to_string(), input.to_string());
    input
}