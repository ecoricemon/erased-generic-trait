use syn::{parse_quote, Ident};

/// Modifies `Ident` name with the given `new_name`.
#[allow(dead_code)]
pub fn modify_ident(ident: &mut Ident, new_name: &str) {
    *ident = Ident::new(new_name, ident.span());
}

/// Clones `ident` with the given `new_name`.
#[allow(dead_code)]
pub fn clone_ident_with_name(ident: &Ident, new_name: &str) -> Ident {
    Ident::new(new_name, ident.span())
}

/// Clones `ident` with the given `prefix`.
#[allow(dead_code)]
pub fn clone_ident_with_prefix(ident: &Ident, prefix: &str) -> Ident {
    let new_name: String = prefix.chars().chain(ident.to_string().chars()).collect();
    clone_ident_with_name(ident, &new_name)
}

/// Clones `ident` with the given `suffix`.
#[allow(dead_code)]
pub fn clone_ident_with_suffix(ident: &Ident, suffix: &str) -> Ident {
    let mut new_name = ident.to_string();
    new_name.push_str(suffix);
    clone_ident_with_name(ident, &new_name)
}

/// Generates a new `Ident` with the given `name` and dummy `Span`.
#[allow(dead_code)]
pub fn gen_ident(name: &str) -> Ident {
    Ident::new(name, proc_macro2::Span::call_site())
}

/// Gets ident and mutability of `FnArg`.
#[allow(dead_code)]
pub fn parse_arg(arg: &syn::FnArg) -> (&Ident, &syn::Type, Option<&syn::token::Mut>) {
    match arg {
        syn::FnArg::Typed(syn::PatType { ty, pat, .. }) => {
            let ident = match pat.as_ref() {
                syn::Pat::Ident(syn::PatIdent { ident, .. }) => ident,
                _ => unimplemented!(),
            };
            let mutability = match ty.as_ref() {
                // & or &mut
                syn::Type::Reference(syn::TypeReference { mutability, .. }) => mutability,
                // Not a reference
                _ => &None,
            }
            .as_ref();
            (ident, ty.as_ref(), mutability)
        }
        syn::FnArg::Receiver(syn::Receiver { ty, mutability, .. }) => {
            let ident = match ty.as_ref() {
                // & or &mut
                syn::Type::Reference(syn::TypeReference { elem, .. }) => match elem.as_ref() {
                    syn::Type::Path(syn::TypePath { path, .. }) => &path.segments[0].ident,
                    _ => unimplemented!(),
                },
                // Not a reference
                _ => unimplemented!(),
            };
            (ident, ty, mutability.as_ref())
        }
    }
}

/// Gets generic symbols like *T* from the `Generics`.
#[allow(dead_code)]
pub fn get_generic_symbols(generics: &syn::Generics) -> Vec<String> {
    generics.params.iter().map(get_generic_symbol).collect()
}

/// Gets generic symbol like *T* from the `GenericParam`.
#[allow(dead_code)]
pub fn get_generic_symbol(param: &syn::GenericParam) -> String {
    match param {
        syn::GenericParam::Type(syn::TypeParam { ident, .. }) => ident.to_string(),
        _ => unimplemented!(),
    }
}

/// Changes generic reference parameters to `dyn Any` refererences.
#[allow(dead_code)]
pub fn change_arg_to_any<'a>(
    arg: &mut syn::FnArg,
    mut targets: impl Iterator<Item = &'a str>,
) -> Result<(), ()> {
    // Gets into `PatType`
    let ty_dest = match arg {
        syn::FnArg::Typed(syn::PatType { ty, .. }) => ty,
        // Doesn't handle `Receiver`.
        syn::FnArg::Receiver(..) => return Err(()),
    };

    // Gets into `TypeReference`
    let (mutability, ty) = match ty_dest.as_mut() {
        // &T or &mut T
        syn::Type::Reference(syn::TypeReference {
            mutability, elem, ..
        }) => (mutability, elem),
        // Doesn't support other types for now.
        _ => return Err(()),
    };

    // Skips not a generic reference.
    let first_seg = match ty.as_mut() {
        syn::Type::Path(syn::TypePath {
            path: syn::Path { segments, .. },
            ..
        }) => &mut segments[0],
        _ => unimplemented!(),
    };
    let ty = first_seg.ident.to_string();
    if targets.all(|t| ty.as_str() != t) {
        return Err(());
    }

    // Changes the `ty_dest` with `&mut dyn Any` or `&dyn Any`.
    let new_type: syn::TypeReference = match mutability {
        // &mut T
        Some(_) => parse_quote! { &mut dyn std::any::Any },
        // &
        None => parse_quote! { &dyn std::any::Any },
    };
    *ty_dest = Box::new(syn::Type::Reference(new_type));
    Ok(())
}

/// Changes generic reference parameters to `dyn Any` refererences.
#[allow(dead_code)]
pub fn change_args_to_anys<'a>(
    args: impl Iterator<Item = &'a mut syn::FnArg>,
    targets: impl Iterator<Item = &'a str> + Clone,
) {
    for arg in args {
        let _ = change_arg_to_any(arg, targets.clone());
    }
}

/// Gets `Ident` if `FnArg` has the name of `target` as its type's `Ident`.
#[allow(dead_code)]
pub fn get_matched_ident<'a>(arg: &'a syn::FnArg, target: &str) -> Option<&'a Ident> {
    let first_seg = get_nth_pathseg(arg, 0)?;
    let ty = first_seg.ident.to_string();
    if ty.as_str() == target {
        let (ident, _, _) = parse_arg(arg);
        Some(ident)
    } else {
        None
    }
}

/// Gets `PathSegment`s from the given `FnArg(PatType->TypeReference->TypePath)`.
#[allow(dead_code)]
pub fn get_path_segments(
    arg: &syn::FnArg,
) -> Option<&syn::punctuated::Punctuated<syn::PathSegment, syn::Token![::]>> {
    match unwrap_pattype_typereference(arg)? {
        syn::Type::Path(syn::TypePath {
            path: syn::Path { segments, .. },
            ..
        }) => Some(segments),
        _ => None,
    }
}

/// Gets `TypeParamBound`s from the given `FnArg(PatType->TypeReference->TypeTraitObject)`.
#[allow(dead_code)]
pub fn get_trait_bounds(
    arg: &syn::FnArg,
) -> Option<&syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>> {
    match unwrap_pattype_typereference(arg)? {
        syn::Type::TraitObject(syn::TypeTraitObject { bounds, .. }) => Some(bounds),
        _ => None,
    }
}

/// Unwraps `PatType` and `TypeReference`.
#[allow(dead_code)]
pub fn unwrap_pattype_typereference(arg: &syn::FnArg) -> Option<&syn::Type> {
    // Gets into `PatType`
    let ty = match arg {
        syn::FnArg::Typed(syn::PatType { ty, .. }) => ty,
        // Doesn't handle `Receiver`.
        syn::FnArg::Receiver(..) => return None,
    };

    // Gets into `TypeReference`
    let ty = match ty.as_ref() {
        // &T or &mut T
        syn::Type::Reference(syn::TypeReference { elem, .. }) => elem,
        // Doesn't support other types for now.
        _ => return None,
    };

    Some(ty.as_ref())
}

/// Gets nth `PathSegment` from the given `FnArg(PatType->TypeReference->TypePath)`.
#[allow(dead_code)]
pub fn get_nth_pathseg(arg: &syn::FnArg, n: usize) -> Option<&syn::PathSegment> {
    get_path_segments(arg)?.iter().nth(n)
}

/// Makes a generic method become non-generic.
#[allow(dead_code)]
pub fn modify_signature_to_erased(sig: &mut syn::Signature) {
    // Modifies fn names.
    let new_name = format!("erased_{}", sig.ident.to_string().as_str());
    modify_ident(&mut sig.ident, new_name.as_str());

    // Skips if non-generic.
    if !is_generic(sig) {
        return;
    }

    // Gets generic symbols.
    let symbols = get_generic_symbols(&sig.generics);

    // Remove the `Generics` now.
    remove_generics(&mut sig.generics);

    // Change generic symbols in parameters into `dyn Any`.
    let mut results = Vec::new();
    for arg in sig.inputs.iter_mut() {
        results.push(change_arg_to_any(arg, symbols.iter().map(|s| s.as_str())));
    }

    // Check out if there was no generic usage.
    if results.iter().all(|res| res.is_err()) {
        unimplemented!()
    }
}

/// Removes `Generics`.
#[allow(dead_code)]
pub fn remove_generics(generics: &mut syn::Generics) {
    *generics = syn::Generics {
        lt_token: None,
        params: syn::punctuated::Punctuated::new(),
        gt_token: None,
        where_clause: None,
    }
}

/// Gets `Ident`s from the list of `FnArg`.
#[allow(dead_code)]
pub fn get_idents(args: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>) -> Vec<Ident> {
    args.iter().map(|arg| get_ident(arg).clone()).collect()
}

/// Gets a `Ident` from the `FnArg`.
#[allow(dead_code)]
pub fn get_ident(arg: &syn::FnArg) -> &Ident {
    let (ident, _, _) = parse_arg(arg);
    ident
}

/// Gets matched `Ident`s with the given nth generic parameter from the list of `FnArg`.
#[allow(dead_code)]
pub fn get_nth_ident(sig: &syn::Signature, n: usize) -> Vec<Ident> {
    let symbols = get_generic_symbols(&sig.generics);
    let symbol = symbols.into_iter().nth(n);
    if let Some(symbol) = symbol {
        sig.inputs
            .iter()
            .filter_map(|arg| get_matched_ident(arg, symbol.as_str()).cloned())
            .collect()
    } else {
        Vec::new()
    }
}

/// Generates camel case string from the given snake case str.
#[allow(dead_code)]
pub fn camel_case(snake_case: &str) -> String {
    let mut res = String::new();
    let mut capital = false;
    for c in "_".chars().chain(snake_case.chars()) {
        if c == '_' {
            capital = true;
        } else if capital {
            res.push(c.to_ascii_uppercase());
            capital = false;
        } else {
            res.push(c);
        }
    }
    res
}

/// Gets the nth `GenericParam` from the given `Signature`s.
#[allow(dead_code)]
pub fn get_nth_generic(sig: &syn::Signature, n: usize) -> Option<&syn::GenericParam> {
    sig.generics.params.iter().nth(n)
}

/// Determines that the given `Signature` is generic.
#[allow(dead_code)]
pub fn is_generic(sig: &syn::Signature) -> bool {
    get_nth_generic(sig, 0).is_some()
}

/// Determines that the given `FnArg` is a reference of Any.
#[allow(dead_code)]
pub fn is_any(arg: &syn::FnArg) -> bool {
    let bounds = get_trait_bounds(arg);
    if let Some(bounds) = bounds {
        if let Some(bound) = bounds.iter().next() {
            let path = match bound {
                syn::TypeParamBound::Trait(syn::TraitBound { path, .. }) => path,
                _ => unimplemented!(),
            };
            let last_ident = path.segments.last().unwrap().ident.to_string();
            &last_ident == "Any"
        } else {
            false
        }
    } else {
        false
    }
}
