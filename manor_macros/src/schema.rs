use convert_case::{Case, Casing};
use darling::{FromMeta, ast::NestedMeta, util::IdentString};
use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Ident, ItemStruct, Type, TypePath, Visibility, parse::Parser, punctuated::Punctuated,
    token::Comma,
};

use crate::util::catch;

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
struct SchemaArgs {
    collection: Option<String>,
    id_alias: Option<String>,
    schema_name: Option<IdentString>,
    builder_name: Option<IdentString>,
}

pub(crate) fn generate_schema(_args: TokenStream, _input: TokenStream) -> TokenStream {
    let attr_args = catch!(NestedMeta::parse_meta_list(_args.into()));

    let input = syn::parse_macro_input!(_input as ItemStruct);
    let fields = if let syn::Fields::Named(named_fields) = input.fields {
        named_fields
    } else {
        return TokenStream::from(
            darling::Error::unsupported_shape("The provided struct must have named fields.")
                .write_errors(),
        );
    };

    let args = catch!(SchemaArgs::from_list(&attr_args));

    let schema_name = args.schema_name.unwrap_or(input.ident.clone().into());
    let builder_name = args
        .builder_name
        .and_then(|n| Some(n.as_str().to_string()))
        .unwrap_or(format!("{}Builder", schema_name.as_str()));
    let id_alias = IdentString::new(catch!(Ident::from_string(
        &args.id_alias.unwrap_or(String::from("id"))
    )));
    let collection_name = args
        .collection
        .unwrap_or(schema_name.as_str().to_string())
        .to_case(Case::Snake);

    let mut new_fields: Punctuated<syn::Field, Comma> = Punctuated::new();
    let mut has_id = false;

    for field in fields.named {
        let field_ident = IdentString::new(field.ident.clone().unwrap());

        if field_ident == id_alias {
            if field.ty == Type::Path(TypePath::from_string("manor::bson::oid::ObjectId").unwrap())
            {
                has_id = true;
                new_fields.push(catch!(
                    syn::Field::parse_named.parse(
                        quote! {
                            #[serde(rename = "_id", default = "manor::bson::oid::ObjectId::new")]
                            #[builder(default = "manor::bson::oid::ObjectId::new()")]
                            pub #id_alias: manor::bson::oid::ObjectId
                        }
                        .into()
                    )
                ));
            } else {
                return TokenStream::from(
                    darling::Error::unexpected_type("Expected the ID field to be an ObjectId")
                        .write_errors(),
                );
            }
        } else {
            let mut new_field = field.clone();
            println!("{}", new_field.clone().into_token_stream().to_string());
            new_field.vis = Visibility::from_string("pub").unwrap();
            new_fields.push(new_field);
        }
    }

    if !has_id {
        new_fields.insert(
            0,
            catch!(
                syn::Field::parse_named.parse(
                    quote! {
                        #[serde(rename = "_id", default = "manor::bson::oid::ObjectId::new")]
                        #[builder(default = "manor::bson::oid::ObjectId::new()")]
                        pub #id_alias: manor::bson::oid::ObjectId
                    }
                    .into()
                )
            ),
        );
    }

    new_fields.push(catch!(
        syn::Field::parse_named.parse(
            quote! {
                #[serde(skip)]
                #[builder(setter(name = "collection"), default = "None")]
                _collection: Option<manor::Collection<#schema_name>>
            }
            .into()
        )
    ));

    let assembled_fields = new_fields.into_token_stream();

    quote! {
        use manor::derive_builder;

        #[derive(Clone, Debug, manor::serde::Serialize, manor::serde::Deserialize, manor::derive_builder::Builder)]
        #[builder(name = #builder_name, crate = "manor::derive_builder", setter(into, strip_option))]
        pub struct #schema_name {
            #assembled_fields
        }

        impl manor::Model for #schema_name {
            fn from_document(document: manor::bson::Document, collection: manor::Collection<Self>) -> manor::MResult<Self> {
                let mut created = manor::bson::from_document::<Self>(document).or_else(|e| Err(manor::Error::from(e)))?;
                created._collection = Some(collection);
                Ok(created)
            }
            fn collection_name() -> String {
                #collection_name.to_string()
            }
            fn collection(&self) -> manor::Collection<Self> {
                self._collection.clone().expect("Collection is not initialized.")
            }
            fn id(&self) -> manor::bson::oid::ObjectId {
                self.#id_alias.clone()
            }
        }
    }
    .into()
}
