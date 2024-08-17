use std::sync::OnceLock;

use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use syn::{
    parse_macro_input, AttributeArgs, BareFnArg, Error, FnArg, Ident, ItemFn, Lit, LitStr, Meta,
    NestedMeta, Result, TypeBareFn,
};

enum Address {
    Signature(String),
    Address(usize),
}

struct Args {
    pub address: Address,
}

fn pattern_regex() -> &'static Regex {
    static PATTERN_REGEX: OnceLock<Regex> = OnceLock::new();
    PATTERN_REGEX.get_or_init(|| Regex::new(r"^(([0-9A-Z]{2}|\?)\s)*([0-9A-Z]{2}|\?)$").unwrap())
}

impl Args {
    fn new(args: AttributeArgs) -> Result<Self> {
        let mut address = None;

        for arg in args {
            match arg {
                NestedMeta::Meta(Meta::NameValue(nv)) => {
                    if nv.path.is_ident("pattern") {
                        if address.is_some() {
                            return Err(Error::new_spanned(
                                nv.path,
                                "address has already been specified",
                            ));
                        }

                        if let Lit::Str(lit) = nv.lit {
                            if pattern_regex().is_match(&lit.value()) {
                                address = Some(Address::Signature(lit.value().to_owned()));
                            } else {
                                return Err(Error::new_spanned(
                                    lit,
                                    "`pattern` is invalid, does not match pattern format (`DE ? BE EF`)",
                                ));
                            }
                        } else {
                            return Err(Error::new_spanned(
                                nv.lit,
                                "`pattern` must be literal string",
                            ));
                        }
                    } else if nv.path.is_ident("address") {
                        if address.is_some() {
                            return Err(Error::new_spanned(
                                nv.path,
                                "address has already been specified",
                            ));
                        }

                        if let Lit::Int(lit) = nv.lit {
                            if let Ok(value) = lit.base10_parse() {
                                address = Some(Address::Address(value));
                            } else {
                                return Err(Error::new_spanned(
                                    lit,
                                    "`address` is an invalid integer",
                                ));
                            }
                        } else {
                            return Err(Error::new_spanned(
                                nv.lit,
                                "`pattern` must be literal string",
                            ));
                        }
                    } else {
                        return Err(Error::new_spanned(
                            nv.path.clone(),
                            "unknown attribute".to_string(),
                        ));
                    }
                }
                arg => {
                    return Err(Error::new_spanned(arg, "unknown attribute".to_string()));
                }
            }
        }

        Ok(Self {
            address: address.expect("missing `address` attribute"),
        })
    }
}

#[proc_macro_attribute]
pub fn detour(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Extract arguments
    let args = match Args::new(parse_macro_input!(args as AttributeArgs)) {
        Ok(gen) => gen,
        Err(err) => return err.to_compile_error().into(),
    };

    // Extract input
    let detour = parse_macro_input!(input as ItemFn);
    let visibility = detour.vis.clone();
    let signature = detour.sig.clone();
    let function_name = Ident::new(&signature.ident.to_string(), Span::call_site());
    let detour_name = Ident::new(&function_name.to_string().to_uppercase(), Span::call_site());
    let binder_name = Ident::new(&format!("{}_BINDER", detour_name), Span::call_site());
    let detour_type = TypeBareFn {
        lifetimes: None,
        unsafety: signature.unsafety,
        abi: signature.abi,
        fn_token: signature.fn_token,
        paren_token: signature.paren_token,
        inputs: signature
            .inputs
            .iter()
            .filter_map(|arg| {
                match arg {
                    FnArg::Receiver(_) => {
                        // Probably an error... cannot have a self type in a detour
                        None
                    }
                    FnArg::Typed(typed) => Some(BareFnArg {
                        attrs: typed.attrs.clone(),
                        name: None,
                        ty: *typed.ty.clone(),
                    }),
                }
            })
            .collect(),
        variadic: signature.variadic.clone(),
        output: signature.output.clone(),
    };

    let address_block = match args.address {
        Address::Signature(addr_sig) => {
            let error_string = LitStr::new(
                &format!("failed to find {}", signature.ident),
                Span::call_site(),
            );
            quote! {
                use anyhow::Context;
                let address = module.scan(#addr_sig).context(#error_string)?;
            }
        }
        Address::Address(address) => quote! {
            let address = #address;
        },
    };

    quote! {
        #visibility static #detour_name: std::sync::OnceLock<::re_utilities::retour::GenericDetour<#detour_type>> = std::sync::OnceLock::new();
        #visibility static #binder_name: ::re_utilities::detour_binder::CompiletimeDetourBinder = ::re_utilities::detour_binder::CompiletimeDetourBinder {
            enable: &|| {
                unsafe {
                    if #detour_name.get().is_none() {
                        #address_block
                        #detour_name.set(
                            ::re_utilities::retour::GenericDetour::<#detour_type>::new(
                                ::std::mem::transmute(address),
                                #function_name
                            )?
                        ).expect("detour already bound");
                    }
                    #detour_name.get().expect("detour not bound").enable()?;
                }
                Ok(())
            },
            disable: &|| {
                unsafe {
                    #detour_name.get().expect("detour not bound").disable()?;
                }
                Ok(())
            },
        };

        #detour
    }
    .into()
}
