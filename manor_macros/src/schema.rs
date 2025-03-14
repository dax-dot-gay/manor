use convert_case::{Case, Casing};
use darling::{FromMeta, ast::NestedMeta, util::IdentString};
use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    parse::{Parse, Parser}, punctuated::Punctuated, token::Comma, Expr, Field, Ident, ItemStruct, TypePath
};

use crate::util::catch;

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
struct FieldArgs {
    id: Option<Expr>,
    alias: Option<IdentString>,
}
#[derive(Debug, FromMeta, Default)]
#[darling(default)]
struct SchemaArgs {
    collection: Option<String>,
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
    let formatted_gen_id = format!("{}::gen_id", schema_name.as_str());
    let formatted_gen_id_call = format!("{}::gen_id()", schema_name.as_str());
    let builder_name = args
        .builder_name
        .and_then(|n| Some(n.as_str().to_string()))
        .unwrap_or(format!("{}Builder", schema_name.as_str()));
    let collection_name = args
        .collection
        .unwrap_or(schema_name.as_str().to_string())
        .to_case(Case::Snake);

    let mut new_fields: Punctuated<syn::Field, Comma> = Punctuated::new();
    let mut id_type: syn::Type = syn::Type::Path(catch!(TypePath::from_string("manor::bson::oid::ObjectId")));
    let mut id_generator: syn::Expr = syn::Expr::Path(catch!(syn::ExprPath::parse.parse(quote! {manor::bson::oid::ObjectId::new}.into())));
    let mut id_name: Option<Ident> = None;
    for field in fields.named {
        let mut already_parsed = false;
        for attr in field.attrs.clone() {
            if attr.path().is_ident("field") {
                let parsed_field = catch!(FieldArgs::from_meta(&attr.meta));
                if let Some(id_field) = parsed_field.id.clone() {
                    id_generator = match id_field {
                        Expr::Closure(closure) => {
                            let tokens = closure.to_token_stream();
                            Expr::Paren(syn::ExprParen::parse.parse(quote! {(#tokens)}.into()).unwrap())
                        },
                        Expr::Call(caller) => {
                            let tokens = caller.to_token_stream();
                            Expr::Paren(syn::ExprParen::parse.parse(quote! {(|| #tokens)}.into()).unwrap())
                        },
                        Expr::Path(path) => Expr::Path(path),
                        _ => {
                            return quote! {"ERROR: Expected a closure, function call, or path to a function."}.into();
                        }
                    };

                    id_type = field.ty.clone();
                    id_name = Some(field.ident.clone().unwrap());
                    
                    let id_ident = id_name.clone().unwrap();

                    new_fields.push(catch!(
                        Field::parse_named.parse(
                            quote! {
                                #[serde(rename = "_id", default = #formatted_gen_id)]
                                #[builder(default = #formatted_gen_id_call)]
                                pub #id_ident: #id_type
                            }.into()
                        )
                    ));

                    already_parsed = true;
                } else {
                    
                }
            }
        }

        if !already_parsed {
            new_fields.push(field.clone());
        }
    }

    if id_name.is_none() {
        new_fields.insert(
            0,
            catch!(
                syn::Field::parse_named.parse(
                    quote! {
                        #[serde(rename = "_id", default = #formatted_gen_id)]
                        #[builder(default = #formatted_gen_id_call)]
                        pub id: #id_type
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
    let id_alias = id_name.unwrap_or(catch!(Ident::from_string("id")));

    quote! {
        #[derive(Clone, Debug, manor::serde::Serialize, manor::serde::Deserialize, manor::derive_builder::Builder)]
        #[builder(name = #builder_name, crate = "manor::derive_builder", setter(into, strip_option))]
        pub struct #schema_name {
            #assembled_fields
        }

        impl #schema_name {
            fn gen_id() -> #id_type {
                #id_generator()
            }
        }

        impl manor::Model for #schema_name {
            type Id = #id_type;

            fn from_document(document: manor::bson::Document, collection: Option<manor::Collection<Self>>) -> manor::MResult<Self> {
                let mut created = manor::bson::from_document::<Self>(document).or_else(|e| Err(manor::Error::from(e)))?;
                created._collection = collection.clone();
                Ok(created)
            }
            fn collection_name() -> String {
                #collection_name.to_string()
            }
            fn own_collection(&self) -> Option<manor::Collection<Self>> {
                self._collection.clone()
            }
            fn id(&self) -> Self::Id {
                self.#id_alias.clone()
            }
            fn generate_id() -> Self::Id {
                Self::gen_id()
            }
            fn attach_collection(&mut self, collection: manor::Collection<Self>) -> () {
                self._collection = Some(collection.clone());
            }
        }
    }
    .into()
}
