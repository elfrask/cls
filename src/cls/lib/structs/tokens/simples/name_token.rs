use crate::cls::lib::structs::tokens::{meta::{BaseToken, TokenMeta}, types::TokenTypesNames};

pub struct NameToken {
  pub meta: TokenMeta,
  pub name: String,
}

// impl_base_token!(NameToken);

impl NameToken {
  pub fn new(index: i64, name: &str) -> NameToken {
    NameToken {
      meta: TokenMeta::new(index, TokenTypesNames::Name),
      name: name.to_string(),
    }
  }
}


impl BaseToken for NameToken  {
  fn repr(&self) -> String {
    format!("{}: '{}'", self.meta, self.name)
  }
}