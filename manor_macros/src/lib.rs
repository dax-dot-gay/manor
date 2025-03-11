use proc_macro::TokenStream;

mod model;

#[proc_macro_derive(Model, attributes(manor))]
pub fn derive_model(item: TokenStream) -> TokenStream {
    model::derive_model(item)
}