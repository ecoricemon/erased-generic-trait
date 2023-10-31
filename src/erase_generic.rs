use crate::common::*;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemTrait};

/// Generates a new trait without generic parameters.
/// Then implements input trait for the new trait object.
pub fn erase_generic(attr: TokenStream, item: TokenStream) -> TokenStream {
    let erased_name = attr.to_string();
    let src_trait = parse_macro_input!(item as ItemTrait);
    let mut erased_trait = src_trait.clone();

    // Makes new trait with the name of `erased_trait_name`.
    into_erased_generic(&mut erased_trait, erased_name.as_str());

    // Makes impl of dyn erased generic trait.
    let generic_for_dyn_erased =
        impl_generic_for_dyn_erased(&src_trait, &erased_trait, erased_name.as_str());

    quote! {
        #src_trait
        #erased_trait
        #generic_for_dyn_erased
    }
    .into()
}

/// Makes generic methods become non-generic.
fn into_erased_generic(ast: &mut ItemTrait, new_name: &str) {
    // Modifies the trait name.
    modify_ident(&mut ast.ident, new_name);

    // Gets signatures.
    let sigs = get_signatures(ast);

    // Tries to change signatures.
    for sig in sigs {
        modify_signature_to_erased(sig);
    }
}

/// Gets `Signature`s from the `ItemTrait`.
fn get_signatures(ast: &mut ItemTrait) -> Vec<&mut syn::Signature> {
    ast.items
        .iter_mut()
        .filter_map(|it| match it {
            syn::TraitItem::Fn(syn::TraitItemFn { sig, .. }) => Some(sig),
            _ => None,
        })
        .collect()
}

/// impl generic for dyn erased.
fn impl_generic_for_dyn_erased(
    src: &ItemTrait,
    erased: &ItemTrait,
    trait_object: &str,
) -> proc_macro2::TokenStream {
    let mut src = src.clone();
    let impl_trait = src.ident.clone();
    let trait_object: proc_macro2::TokenStream = trait_object.parse().unwrap();

    // Gets signatures.
    let sigs = get_signatures(&mut src);

    // Makes erased generic method names.
    let erased_methods = sigs.iter().map(|sig| {
        let new_name = format!("erased_{}", sig.ident.to_string().as_str());
        new_name.parse::<proc_macro2::TokenStream>().unwrap()
    });

    // Is a method(has the receiver `self`) or associated method(doesn't have the receiver)?
    let method_type = sigs.iter().map(|sig| match sig.inputs.first() {
        Some(&syn::FnArg::Receiver(..)) => quote! { self. },
        _ => quote! { #trait_object:: },
    });

    // Determines arguments.
    let mut erased = erased.clone();
    let erased_sigs = get_signatures(&mut erased);
    let skips = sigs.iter().map(|sig| match sig.inputs.first() {
        Some(&syn::FnArg::Receiver(..)) => 1_usize,
        _ => 0_usize,
    });
    let args = skips
        .zip(sigs.iter().zip(erased_sigs.iter()))
        .map(|(skip, (sig, esig))| {
            let args = sig
                .inputs
                .iter()
                .zip(esig.inputs.iter())
                .skip(skip)
                .map(|(arg, earg)| {
                    if arg == earg {
                        let ident = parse_arg(arg).0;
                        quote! { #ident }
                    } else {
                        let (ident, _, mutability) = parse_arg(arg);
                        match mutability {
                            Some(_) => parse_quote! { #ident as &mut dyn std::any::Any },
                            None => parse_quote! { #ident as &dyn std::any::Any },
                        }
                    }
                });
            quote! { #(#args),* }
        });

    quote! {
        impl #impl_trait for dyn #trait_object {
            #(
                #[inline]
                #sigs {
                    #method_type #erased_methods ( #args )
                }
            )*
        }
    }
}
