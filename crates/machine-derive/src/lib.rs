//! Procedural macro DSL for machine-rs.

use proc_macro::TokenStream;

mod state_machine;

/// Declarative state machine procedural macro.
///
/// Compiles a high-level transition description into an optimal, zero-allocation
/// state machine kernel implementation (`machine_core::Machine`).
#[proc_macro_attribute]
pub fn state_machine(attr: TokenStream, item: TokenStream) -> TokenStream {
    state_machine::expand_state_machine(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
