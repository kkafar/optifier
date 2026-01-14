//! Proc macro crate: `Partial` derive
//!
//! Derive `Partial` on a struct `Foo` to generate a new struct named `FooPartial`
//! where every field type is wrapped in `Option<T>` unless it is already an `Option<T>`.
//! Only structs with named fields are accepted; tuple and unit structs are not supported **yet**.
//!
//! Example:
//! ```ignore
//! #[derive(Partial)]
//! pub struct Foo {
//!     a: i32,
//!     b: Option<String>,
//!     pub c: Vec<u8>,
//! }
//! ```
//! expands to:
//! ```ignore
//! pub struct FooPartial {
//!     a: Option<i32>,
//!     b: Option<String>, // stays as-is
//!     pub c: Option<Vec<u8>>,
//! }
//! ```

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DeriveInput, FieldsNamed, Ident, ImplGenerics, Path, PathArguments,
    Type, TypeGenerics, TypePath, Visibility, WhereClause,
};

/// Derive macro to generate a `*Partial` variant of a struct with all fields wrapped in `Option`.
///
/// Put `#[derive(optifier::Partial)]` on a struct item. The macro will output:
/// - A new struct `<OriginalName>Partial` with the same visibility and field names, but with each
///   field type wrapped in `Option<T>`, unless it is already `Option<...>`.
///
/// Supported:
/// - Named-field structs
///
/// Not supported:
/// - Tuple structs
/// - Unit structs
/// - Enums
///
/// Notes:
/// - Generic parameters and lifetimes are copied as-is to the generated `Partial` struct.
/// - Field-level visibilities are preserved.
/// - Field attributes are not copied to the generated struct (to avoid duplicating derives/etc.).
#[proc_macro_derive(Partial)]
pub fn derive_partial(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let orig_vis: Visibility = input.vis.clone();
    let orig_ident: Ident = input.ident.clone();
    let partial_ident = format_ident!("{}Partial", orig_ident);
    let maybe_derive_attr = deduce_derive_attr(&input);

    let Data::Struct(input_struct) = input.data else {
        panic!("Optifier supports only struct types");
    };

    // Copy generics from original to partial
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Currently we support only named fields inside a structure.
    // Support for tuple structs will be added in the future.
    //
    // Generate fields for the partial struct by wrapping types in Option if needed

    let syn::Fields::Named(fields) = &input_struct.fields else {
        panic!("Optifier supports only named fields");
    };

    let partial_fields = fields.named.iter().map(|f| {
        let f_vis = &f.vis;
        let f_ident = f
            .ident
            .as_ref()
            .expect("Optifier: Named field must have ident");
        let f_ty = &f.ty;
        if is_option_type(f_ty) {
            quote! {
                #f_vis #f_ident: #f_ty
            }
        } else {
            quote! {
                #f_vis #f_ident: std::option::Option<#f_ty>
            }
        }
    });

    let struct_def = quote! {
        #maybe_derive_attr
        #orig_vis struct #partial_ident #generics #where_clause {
            #(#partial_fields),*
        }
    };

    let merge_function_impl_block = construct_merge_impl_block(
        &partial_ident,
        fields,
        &impl_generics,
        &ty_generics,
        where_clause,
    );

    let generated_code = quote! {
        #struct_def
        #merge_function_impl_block
    };

    TokenStream::from(generated_code)
}

fn deduce_derive_attr(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let mut derives: Vec<proc_macro2::TokenStream> = Vec::new();

    for attr in &input.attrs {
        if attr.path().is_ident("derive") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("Debug") {
                    derives.push(quote! { Debug });
                }
                if meta.path.is_ident("Clone") {
                    derives.push(quote! { Clone });
                }
                Ok(())
            });
        }
    }

    if derives.is_empty() {
        quote! {}
    } else {
        quote! { #[derive( #(#derives),* )] }
    }
}

/// Detect whether a given type is `Option<...>`
///
/// Heuristic:
/// - Must be a `Type::Path`
/// - Last path segment ident equals `Option`
/// - Has angle-bracketed generic arguments
fn is_option_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => is_path_option(path),
        _ => false,
    }
}

fn is_path_option(path: &Path) -> bool {
    if let Some(last) = path.segments.last() {
        last.ident == "Option" && matches!(last.arguments, PathArguments::AngleBracketed(_))
    } else {
        false
    }
}

fn construct_merge_impl_block(
    type_ident: &Ident,
    fields_named: &FieldsNamed,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> proc_macro2::TokenStream {
    let fields_merged = fields_named.named.iter().map(|f| {
        let f_ident = f
            .ident
            .as_ref()
            .expect("Optifier: Named field must have ident");

        quote! {
            #f_ident: self.#f_ident.or(other.#f_ident)
        }
    });

    let merge_function_impl = quote! {
        pub fn merge(self, other: #type_ident) -> Self {
            Self {
                #(#fields_merged),*
            }
        }
    };

    quote! {
        impl #impl_generics #type_ident #ty_generics #where_clause {
            #merge_function_impl
        }
    }
}
