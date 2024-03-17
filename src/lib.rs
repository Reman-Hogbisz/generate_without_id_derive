use proc_macro::{self, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, FieldsNamed, Meta};

#[proc_macro_derive(CreateWithoutId, attributes(id_name, diesel, without_id_ts_type))]
pub fn create_without_id(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);

    let diesel_attrs: Vec<&syn::Attribute> = attrs
        .iter()
        .filter(|attr| attr.path.is_ident("diesel"))
        .collect();

    assert!(
        !diesel_attrs.is_empty(),
        "derive(CreateWithoutId) requires a diesel(table_name = \"...\") attribute (diesel attrs is empty)"
    );

    let table_name_attr = diesel_attrs.into_iter().find(|attr| {
        let tokens = attr.to_token_stream().into_iter().collect::<Vec<_>>();
        tokens
            .iter()
            .any(|token| token.to_string().contains("table_name"))
    });

    assert!(
        table_name_attr.is_some(),
        "derive(CreateWithoutId) requires a diesel(table_name = \"...\") attribute (no table_name attr found)"
    );

    let table_name_attr = table_name_attr.unwrap(); // Safety: We just checked that it is some

    let id_name_attr = attrs.iter().find(|attr| attr.path.is_ident("id_name"));

    let id_name = match id_name_attr {
        Some(attr) => {
            let tokens = attr.to_token_stream().into_iter().collect::<Vec<_>>();
            let id_name = tokens[2].to_string();
            id_name
        }
        None => "id".to_string(),
    };

    let struct_token = match data {
        syn::Data::Struct(s) => s,
        _ => panic!("derive(CreateFilter) only supports structs"),
    };

    let fields = match struct_token.fields {
        syn::Fields::Named(FieldsNamed { named, .. }) => named,
        _ => panic!("derive(CreateFilter) only supports named fields"),
    };

    let fields: Vec<_> = fields
        .iter()
        .filter_map(|f| match f.ident {
            Some(ref i) => Some((i, &f.ty, &f.attrs)),
            None => None,
        })
        .collect();

    let mut filtered_field_declarations = TokenStream2::default();
    let mut into_field_declaration = TokenStream2::default();
    let mut into_ref_field_declaration = TokenStream2::default();

    fields.into_iter().for_each(|(field, ftype, attrs)| {
        if field.to_string() == id_name {
            return;
        }

        if let Some(attr) = attrs
            .iter()
            .find(|attr| attr.path.is_ident("without_id_ts_type"))
        {
            let ts_type = match attr.parse_args() {
                Ok(Meta::NameValue(nv)) => {
                    if nv.path.is_ident("type") {
                        nv.lit
                    } else {
                        panic!("without_id_ts_type must be type = \"...\"")
                    }
                }
                _ => panic!("without_id_ts_type must be type = \"...\""),
            };

            filtered_field_declarations.extend(quote! {
                #[ts(type = #ts_type)]
                pub #field: Option<#ftype>,
            });
        } else {
            filtered_field_declarations.extend(quote! {
                pub #field: Option<#ftype>,
            });
        }

        into_field_declaration.extend::<TokenStream2>(quote! { #field : self.#field, });
        into_ref_field_declaration.extend::<TokenStream2>(quote! { #field : self.#field.clone(), });
    });

    let struct_name = Ident::new(&format!("{}WithoutId", ident), Span::call_site());

    let output = quote! {

        use crate::util::*;
        use crate::db_connection::*;
        use diesel::prelude::*;

        #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Insertable, AsChangeset, TS)]
        #[ts(export)]
        #table_name_attr
        #[diesel(treat_none_as_null = true)]
        pub struct #struct_name {
            #filtered_field_declarations
        } impl Into<#struct_name> for #ident {
            fn into(self) -> #struct_name {
                #struct_name {
                    #into_field_declaration
                }
            }
        } impl Into<#struct_name> for &#ident {
            fn into(self) -> #struct_name {
                #struct_name {
                    #into_ref_field_declaration
                }
            }
        }
    };

    output.into()
}
