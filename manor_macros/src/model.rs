use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Field, parse_macro_input};

#[derive(FromMeta, Default)]
#[darling(default)]
pub(crate) struct SchemaAttrs {
    collection: Option<String>,
    id: Option<String>,
}

#[derive(FromDeriveInput)]
#[darling(
    attributes(manor),
    forward_attrs(allow, doc, cfg),
    supports(struct_named)
)]
pub(crate) struct DeriveModel {
    ident: syn::Ident,
    #[darling(default)]
    schema: SchemaAttrs,
    data: darling::ast::Data<(), Field>,
}

pub(crate) fn derive_model(tokens: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(tokens as DeriveInput);
    let model = match DeriveModel::from_derive_input(&derive_input) {
        Ok(derived) => derived,
        Err(err) => {
            return TokenStream::from(err.write_errors());
        }
    };

    let collection = model
        .schema
        .collection
        .unwrap_or(model.ident.to_string())
        .to_case(Case::Snake);
    let id_field = model.schema.id.unwrap_or(String::from("_id"));
    let schema_name = model.ident;
    let id_field_ident = match syn::Ident::from_string(&id_field) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let schema_impl = quote! {
        impl manor::Schema for #schema_name {
            fn id_field() -> String {
                String::from(#id_field)
            }

            fn collection_name() -> String {
                String::from(#collection)
            }

            fn id(&self) -> manor::bson::oid::ObjectId {
                self.#id_field_ident
            }
        }
    };

    quote! {
        #schema_impl
    }
    .into()
}
