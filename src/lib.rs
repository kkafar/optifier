//! Proc macro crate: `Partial` derive
//!
//! Derive `Partial` on a struct `Foo` to generate a new struct named `FooPartial`
//! where every field type is wrapped in `Option<T>` unless it is already an `Option<T>`.
//! Only structs with named fields are accepted; tuple and unit structs are not supported **yet**.
//!
//! Example:
//! ```ignore
//! #[optifier::partial_derive(Debug, Clone)]
//! #[derive(optifier::Partial)]
//! pub struct Foo {
//!     a: i32,
//!     b: Option<String>,
//!     pub c: Vec<u8>,
//! }
//! ```
//! expands to:
//! ```ignore
//! #[derive(Debug, Clone)]
//! pub struct FooPartial {
//!     a: Option<i32>,
//!     b: Option<String>, // stays as-is
//!     pub c: Option<Vec<u8>>,
//! }
//! ```
//!
//! The `#[optifier::partial_derive(...)]` attribute controls which traits are derived for the
//! generated `*Partial` type. It accepts a comma-separated list of trait paths.

extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, FieldsNamed, Generics, Ident, ImplGenerics, Path, PathArguments, Type,
    TypeGenerics, TypePath, Visibility, WhereClause, parse_macro_input,
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
    let maybe_derive_attr = collect_partial_derives(&input);

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

    let partial_struct_def = construct_partial_struct(
        &partial_ident,
        &orig_vis,
        fields,
        &generics,
        where_clause,
        maybe_derive_attr,
    );

    let merge_function_impl_block = construct_merge_impl_block(
        &partial_ident,
        fields,
        &impl_generics,
        &ty_generics,
        where_clause,
    );

    let tryfrom_impl_block = construct_tryfrom_impl_block(
        &orig_ident,
        &partial_ident,
        fields,
        &impl_generics,
        &ty_generics,
        where_clause,
    );

    // FIXME: I've got no idea why adding a semicolon after #merge_function_impl_block
    // fixes the compilation error, but it does. Seems to me that semicolon is not required
    // after end of impl block. Need to investigate further.
    let generated_code = quote! {
        #partial_struct_def
        #merge_function_impl_block
        #tryfrom_impl_block
    };

    TokenStream::from(generated_code)
}

/// Attribute macro to configure derives for the generated `*Partial` type.
///
/// Usage:
/// ```ignore
/// #[optifier::partial_derive(Debug, Clone)]
/// #[derive(optifier::Partial)]
/// struct Foo { /* ... */ }
/// ```
///
/// This will cause the generated `FooPartial` to have:
/// ```ignore
/// #[derive(Debug, Clone)]
/// struct FooPartial { /* ... */ }
/// ```
#[proc_macro_attribute]
pub fn partial_derive(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // This attribute is intentionally a no-op at expansion time.
    // The `Partial` derive macro will read the attribute arguments
    // from the original item via `collect_partial_derives`.
    item
}

fn collect_partial_derives(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let mut derives: Vec<proc_macro2::TokenStream> = Vec::new();

    // Look for: #[optifier::partial_derive(Debug, Clone, ...)]
    for attr in &input.attrs {
        if !attr.path().is_ident("partial_derive") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            let path = &meta.path;
            derives.push(quote! { #path });
            Ok(())
        });
    }

    if derives.is_empty() {
        // No #[partial_derive(...)] found -> no derives for the partial type
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

fn construct_partial_struct(
    type_ident: &Ident,
    type_vis: &Visibility,
    fields_named: &FieldsNamed,
    generics: &Generics,
    where_clause: Option<&WhereClause>,
    derive_attrs: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let partial_fields = fields_named.named.iter().map(|f| {
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
                #f_vis #f_ident: ::std::option::Option<#f_ty>
            }
        }
    });

    let partial_struct_def = quote! {
        #derive_attrs
        #type_vis struct #type_ident #generics #where_clause {
            #(#partial_fields),*
        }
    };

    quote! {
        #partial_struct_def
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

/// Construct the error type and `TryFrom<Partial> for Original` implementation.
///
/// - The error type is `<OriginalName>PartialError`.
/// - It has one variant per non-`Option` field in the original struct.
/// - Conversion succeeds only if all non-optional fields are present (`Some`) in the partial.
fn construct_tryfrom_impl_block(
    orig_ident: &Ident,
    partial_ident: &Ident,
    fields_named: &FieldsNamed,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> proc_macro2::TokenStream {
    // Name of the error type: e.g. FooPartialError
    let error_ident = format_ident!("{}Error", partial_ident);

    // One enum variant per non-Option field, e.g. AMissing, FieldNameMissing, etc.
    let error_variants = fields_named.named.iter().filter_map(|f| {
        let f_ident = f
            .ident
            .as_ref()
            .expect("Optifier: Named field must have ident");
        let f_ty = &f.ty;

        if is_option_type(f_ty) {
            // Original field was already Option<...> → absence is allowed, no error variant
            return None;
        }

        // Variant name: <FieldName>Missing, using PascalCase.
        // Example: "a" -> "AMissing", "user_id" -> "UserIdMissing".
        let f_name_str = f_ident.to_string();
        let f_name_in_pascal_case = f_name_str.to_case(Case::Pascal);
        let variant_name = format!("{}Missing", f_name_in_pascal_case);
        let variant_ident = format_ident!("{}", variant_name);

        Some(quote! {
            #[error("Field `{}` is missing", #f_name_str)]
            #variant_ident
        })
    });

    // For constructing the original struct, we need, per field:
    //
    // - If original type was non-Option: self.field.ok_or(ErrorVariant)?
    // - If original type was Option: self.field (already Option<T>)
    let construct_fields = fields_named.named.iter().map(|f| {
        let f_ident = f
            .ident
            .as_ref()
            .expect("Optifier: Named field must have ident");
        let f_ty = &f.ty;

        if is_option_type(f_ty) {
            // Accept the value as-is
            quote! {
                #f_ident: partial.#f_ident
            }
        } else {
            // Reconstruct the variant name in the same way (PascalCase + "Missing")
            let raw_name = f_ident.to_string();
            let pascal = raw_name.to_case(Case::Pascal);
            let variant_name = format!("{}Missing", pascal);
            let variant_ident = format_ident!("{}", variant_name);

            quote! {
                #f_ident: partial.#f_ident.ok_or(#error_ident::#variant_ident)?
            }
        }
    });

    // We require thiserror; users must have it in their Cargo.toml.
    // We fully qualify the path so they don't need to `use` it in their own module.
    let error_def = quote! {
        #[derive(::thiserror::Error, Debug)]
        pub enum #error_ident {
            #(#error_variants),*
        }
    };

    let try_from_impl = quote! {
        impl #impl_generics ::std::convert::TryFrom<#partial_ident #ty_generics> for #orig_ident #ty_generics #where_clause {
            type Error = #error_ident;

            fn try_from(partial: #partial_ident #ty_generics) -> ::std::result::Result<#orig_ident #ty_generics, Self::Error> {
                Ok(#orig_ident {
                    #(#construct_fields),*
                })
            }
        }
    };

    quote! {
        #error_def
        #try_from_impl
    }
}
