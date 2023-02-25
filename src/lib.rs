use proc_macro::{self, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, FieldsNamed};

#[proc_macro_derive(CreateWithoutId, attributes(changeset_options, id_name, table_name))]
pub fn create_without_id(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);

    let changeset_options_attr = match attrs
        .iter()
        .find(|attr| attr.path.is_ident("changeset_options"))
    {
        Some(attr) => attr,
        None => panic!("derive(CreateWithoutId) requires a changeset_options attribute"),
    };

    let table_name_attr = match attrs.iter().find(|attr| attr.path.is_ident("table_name")) {
        Some(attr) => attr,
        None => panic!("derive(CreateWithoutId) requires a table_name attribute"),
    };

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

    let (idents, types): (Vec<_>, Vec<_>) = fields
        .iter()
        .filter_map(|f| match f.ident {
            Some(ref i) => Some((i, &f.ty)),
            None => None,
        })
        .unzip();

    let mut filtered_field_declarations = TokenStream2::default();

    idents
        .into_iter()
        .zip(types.into_iter())
        .for_each(|(field, ftype)| {
            if field.to_string() == id_name {
                return;
            }

            filtered_field_declarations.extend::<TokenStream2>(quote! { pub #field : #ftype, });
        });

    let struct_name = Ident::new(&format!("{}WithoutId", ident), Span::call_site());

    let output = quote! {

        use crate::util::*;
        use crate::db_connection::*;
        use diesel::prelude::*;

        #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Insertable, AsChangeset)]
        #table_name_attr
        #changeset_options_attr
        pub struct #struct_name {
            #filtered_field_declarations
        }
    };

    output.into()
}
