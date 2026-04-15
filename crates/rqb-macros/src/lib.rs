//! Derive macros for rqb.

use proc_macro::TokenStream;
use syn::{DeriveInput, Error, parse_macro_input};

mod attrs;
mod field;
mod model;
mod write_record;

#[proc_macro_derive(WriteRecord, attributes(rqb))]
/// Derives `rqb::WriteRecord` for insert and update DTOs.
pub fn derive_write_record(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    write_record::expand(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
