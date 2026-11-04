use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    ItemImpl, Result, Token, Type,
    parse::{Parse, ParseStream},
};

/// Attributes passed to `#[state_machine(...)]`.
pub struct StateMachineArgs {
    pub yield_ty: Type,
    pub resume_ty: Type,
    pub return_ty: Type,
}

impl Parse for StateMachineArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut yield_ty = None;
        let mut resume_ty = None;
        let mut return_ty = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if key == "Yield" {
                if yield_ty.is_some() {
                    return Err(syn::Error::new(key.span(), "duplicate `Yield` parameter"));
                }
                yield_ty = Some(input.parse::<Type>()?);
            } else if key == "Resume" {
                if resume_ty.is_some() {
                    return Err(syn::Error::new(key.span(), "duplicate `Resume` parameter"));
                }
                resume_ty = Some(input.parse::<Type>()?);
            } else if key == "Return" {
                if return_ty.is_some() {
                    return Err(syn::Error::new(key.span(), "duplicate `Return` parameter"));
                }
                return_ty = Some(input.parse::<Type>()?);
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown attribute `{key}`, expected `Yield`, `Resume`, or `Return`"),
                ));
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let yield_ty = yield_ty.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing required `Yield = ...` attribute in #[state_machine]",
            )
        })?;
        let resume_ty = resume_ty.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing required `Resume = ...` attribute in #[state_machine]",
            )
        })?;
        let return_ty = return_ty.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing required `Return = ...` attribute in #[state_machine]",
            )
        })?;

        Ok(Self {
            yield_ty,
            resume_ty,
            return_ty,
        })
    }
}

pub fn expand_state_machine(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let args: StateMachineArgs = syn::parse2(attr)?;

    // Try parsing as ItemImpl first: `impl Fsm { fn step(...) ... }`
    if let Ok(mut item_impl) = syn::parse2::<ItemImpl>(item.clone()) {
        let self_ty = &item_impl.self_ty;
        let (impl_generics, _ty_generics, where_clause) = item_impl.generics.split_for_impl();

        let yield_ty = &args.yield_ty;
        let resume_ty = &args.resume_ty;
        let return_ty = &args.return_ty;

        // Check if there is an explicit `transition` or `step` method inside item_impl
        let mut transition_fn = None;
        for item in &mut item_impl.items {
            if let syn::ImplItem::Fn(method) = item {
                if method.sig.ident == "transition" || method.sig.ident == "step" {
                    transition_fn = Some(method.clone());
                    break;
                }
            }
        }

        let machine_impl = if let Some(method) = transition_fn {
            let fn_name = &method.sig.ident;
            quote! {
                impl #impl_generics ::machine_core::Machine for #self_ty #where_clause {
                    type Yield = #yield_ty;
                    type Resume = #resume_ty;
                    type Return = #return_ty;

                    #[inline(always)]
                    fn step(&mut self, resume: Self::Resume) -> ::machine_core::Step<Self::Yield, Self::Return> {
                        self.#fn_name(resume)
                    }
                }
            }
        } else {
            return Err(syn::Error::new_spanned(
                &item_impl,
                "#[state_machine] impl block must contain either a `step` or `transition` method",
            ));
        };

        let expanded = quote! {
            #item_impl

            #machine_impl
        };
        return Ok(expanded);
    }

    // Otherwise, try parsing as ItemEnum: `enum State { ... }`
    if let Ok(item_enum) = syn::parse2::<syn::ItemEnum>(item) {
        let enum_name = &item_enum.ident;
        let enum_vis = &item_enum.vis;
        let (impl_generics, ty_generics, where_clause) = item_enum.generics.split_for_impl();

        let yield_ty = &args.yield_ty;
        let resume_ty = &args.resume_ty;
        let return_ty = &args.return_ty;

        let has_terminal = item_enum.variants.iter().any(|v| v.ident == "Terminal");

        let mut clean_variants = Vec::new();
        for v in &item_enum.variants {
            clean_variants.push(v.clone());
        }

        if !has_terminal {
            let terminal_variant: syn::Variant = syn::parse_quote!(Terminal);
            clean_variants.push(terminal_variant);
        }

        let attrs = &item_enum.attrs;

        let expanded = quote! {
            #(#attrs)*
            #enum_vis enum #enum_name #ty_generics #where_clause {
                #(#clean_variants),*
            }

            impl #impl_generics ::machine_core::Machine for #enum_name #ty_generics #where_clause {
                type Yield = #yield_ty;
                type Resume = #resume_ty;
                type Return = #return_ty;

                #[inline]
                fn step(&mut self, resume: Self::Resume) -> ::machine_core::Step<Self::Yield, Self::Return> {
                    if matches!(self, Self::Terminal) {
                        panic!("illegal attempt to step a terminal state machine: machine already completed");
                    }

                    self.transition(resume)
                }
            }
        };

        return Ok(expanded);
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "#[state_machine] can only be applied to an `enum` definition or an `impl` block",
    ))
}
