use lazy_static::lazy_static;
use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use syn::{
    self, parse_macro_input, AttributeArgs, BareFnArg, Error, FnArg, Ident, ItemFn, Lit, LitStr,
    Meta, NestedMeta, Result, TypeBareFn,
};

enum Address {
    Signature(String),
    Address(usize),
}

struct Args {
    pub name: syn::LitStr,
    pub address: Address,
}

lazy_static! {
    static ref PATTERN_REGEX: Regex =
        Regex::new(r"^(([0-9A-Z]{2}|\?)\s)*([0-9A-Z]{2}|\?)$").unwrap();
}

impl Args {
    fn new(args: AttributeArgs) -> Result<Self> {
        let mut name = None;
        let mut address = None;

        for arg in args {
            match arg {
                NestedMeta::Meta(Meta::NameValue(nv)) => {
                    if nv.path.is_ident("name") {
                        if let Lit::Str(lit) = nv.lit {
                            name = Some(lit.clone());
                        } else {
                            return Err(Error::new_spanned(
                                nv.lit,
                                "`name` must be literal string",
                            ));
                        }
                    } else if nv.path.is_ident("pattern") {
                        if address.is_some() {
                            return Err(Error::new_spanned(
                                nv.path,
                                "address has already been specified",
                            ));
                        }

                        if let Lit::Str(lit) = nv.lit {
                            if PATTERN_REGEX.is_match(&lit.value()) {
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
            name: name.expect("missing `name` attribute"),
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
    let error_string = LitStr::new(
        &format!("failed to find {}", args.name.value()),
        Span::call_site(),
    );
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

    // Remove the ABI from the detour-handler, detour-rs handles that for us
    let detour = ItemFn {
        sig: syn::Signature {
            abi: None,
            ..signature
        },
        ..detour
    };

    let address_block = match args.address {
        Address::Signature(signature) => quote! {
            use anyhow::Context;
            let address = module.scan(#signature).context(#error_string)?;
        },
        Address::Address(address) => quote! {
            let address = #address;
        },
    };

    quote! {
        static_detour! {
            #visibility static #detour_name: #detour_type;
        }

        #visibility static #binder_name: ::re_utilities::detour_binder::StaticDetourBinder = ::re_utilities::detour_binder::StaticDetourBinder {
            bind: &|module| {
                unsafe {
                    #address_block
                    #detour_name.initialize(::std::mem::transmute(address), #function_name)?;
                }
                Ok(())
            },
            enable: &|| {
                unsafe {
                    #detour_name.enable()?;
                }
                Ok(())
            },
            disable: &|| {
                unsafe {
                    #detour_name.disable()?;
                }
                Ok(())
            },
        };

        #detour
    }
    .into()
}
