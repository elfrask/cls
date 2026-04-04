use crate::cls::lib::structs::tokens::{meta::{BaseToken, TokenMeta}, types::TokenTypesNames};

pub struct SymbolToken {
  pub meta: TokenMeta,
  pub symbol: char,
}

// impl_base_token!(SymbolToken);

impl SymbolToken {
  pub fn new(index: i64, symbol: char) -> SymbolToken {
    SymbolToken {
      meta: TokenMeta::new(index, TokenTypesNames::Symbol),
      symbol
    }
  }
}


impl BaseToken for SymbolToken  {
  fn repr(&self, prefix: &str) -> String {
    format!("{}: '{}'", self.meta, self.symbol)
  }
}