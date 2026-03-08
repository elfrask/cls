use crate::{cls::lib::structs::tokens::{meta::TokenMeta, types::TokenTypesNames}, impl_base_token};

pub struct SymbolToken {
  pub meta: TokenMeta,
  pub symbol: char,
}

impl_base_token!(SymbolToken);

impl SymbolToken {
  pub fn new(index: i64, symbol: char) -> SymbolToken {
    SymbolToken {
      meta: TokenMeta::new(index, TokenTypesNames::Operator),
      symbol
    }
  }
}
