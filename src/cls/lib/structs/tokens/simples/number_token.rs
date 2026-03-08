use crate::{cls::lib::structs::tokens::{meta::TokenMeta, types::TokenTypesNames}, impl_base_token};

pub struct NumberToken {
  pub meta: TokenMeta,
  pub is_float: bool,
  pub value: i64,
}

impl_base_token!(NumberToken);

impl NumberToken {
  pub fn newInt(index: i64, value: i64) -> NumberToken {
    NumberToken{
      meta: TokenMeta { index, token_type: TokenTypesNames::Number },
      is_float: false,
      value
    }
  }
  pub fn newFloat(index: i64, value: f64) -> NumberToken {
    NumberToken{
      meta: TokenMeta { index, token_type: TokenTypesNames::Number },
      is_float: false,
      value: value.to_bits() as i64,
    }
  }
  
  pub fn get_float(&self) -> f64 {
    f64::from_bits(self.value as u64)
  }
}