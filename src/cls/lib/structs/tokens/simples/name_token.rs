use crate::{cls::lib::structs::tokens::{meta::TokenMeta, types::TokenTypesNames}, impl_base_token};

pub struct NameToken {
  pub meta: TokenMeta,
  pub name: String,
}

impl_base_token!(NameToken);

impl NameToken {
  pub fn new(index: i64, name: &str) -> NameToken {
    NameToken {
      meta: TokenMeta::new(index, TokenTypesNames::Name),
      name: name.to_string(),
    }
  }
}
