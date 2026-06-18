use std::sync::OnceLock;

use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use syn::{
    parse_macro_input, punctuated::Punctuated, BareFnArg, Error, Expr, ExprAssign, ExprLit,
    ExprPath, FnArg, Ident, ItemFn, Lit, LitStr, Result, Token, TypeBareFn,
};

enum Address {
    Signature(String),
    /// An arbitrary expression evaluating to a `usize` address: an integer literal
    /// (`0x1234`) or a path to a constant (`some::module::Type::FN_ADDRESS`).
    Address(Box<Expr>),
}

struct Args {
    pub address: Address,
}

fn pattern_regex() -> &'static Regex {
    static PATTERN_REGEX: OnceLock<Regex> = OnceLock::new();
    PATTERN_REGEX.get_or_init(|| Regex::new(r"^(([0-9A-Z]{2}|\?)\s)*([0-9A-Z]{2}|\?)$").unwrap())
}

impl Args {
    fn new(args: Punctuated<Expr, Token![,]>) -> Result<Self> {
        let mut address = None;

        for arg in args {
            let Expr::Assign(ExprAssign { left, right, .. }) = arg else {
                return Err(Error::new_spanned(arg, "expected `name = value`"));
            };
            let Expr::Path(ExprPath { path, .. }) = left.as_ref() else {
                return Err(Error::new_spanned(&left, "expected an attribute name"));
            };

            if path.is_ident("pattern") {
                if address.is_some() {
                    return Err(Error::new_spanned(
                        path,
                        "address has already been specified",
                    ));
                }

                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(lit), ..
                }) = right.as_ref()
                {
                    if pattern_regex().is_match(&lit.value()) {
                        address = Some(Address::Signature(lit.value()));
                    } else {
                        return Err(Error::new_spanned(
                            lit,
                            "`pattern` is invalid, does not match pattern format (`DE ? BE EF`)",
                        ));
                    }
                } else {
                    return Err(Error::new_spanned(
                        &right,
                        "`pattern` must be a literal string",
                    ));
                }
            } else if path.is_ident("address") {
                if address.is_some() {
                    return Err(Error::new_spanned(
                        path,
                        "address has already been specified",
                    ));
                }

                // Accept any expression evaluating to a `usize`: an integer literal
                // (`0x1234`) or a path to a constant (`Type::FN_ADDRESS`).
                address = Some(Address::Address(right));
            } else {
                return Err(Error::new_spanned(path, "unknown attribute"));
            }
        }

        Ok(Self {
            address: address
                .ok_or_else(|| Error::new(Span::call_site(), "missing `address` attribute"))?,
        })
    }
}

#[proc_macro_attribute]
pub fn detour(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Extract arguments
    let args = match Args::new(parse_macro_input!(
        args with Punctuated::<Expr, Token![,]>::parse_terminated
    )) {
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
                let address = module.scan(#addr_sig).map_err(|e| {
                    match e {
                        ::re_utilities::Error::PatternScanFailed { context } => {
                            ::re_utilities::Error::PatternScanFailed {
                                context: Some(format!(
                                    "{}: {}",
                                    #error_string,
                                    context.unwrap_or_default()
                                )),
                            }
                        }
                        other => other,
                    }
                })?;
            }
        }
        Address::Address(expr) => {
            let expr = *expr;
            quote! {
                let address: usize = #expr;
            }
        }
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
