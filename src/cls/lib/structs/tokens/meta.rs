use crate::cls::lib::structs::tokens::types::TokenTypesNames;

pub trait BaseToken {
  fn repr(&self) -> String;
  // fn repr(&self) -> &'static str;
}


pub struct TokenMeta {
  pub index: i64,
  pub token_type: TokenTypesNames,
}

impl TokenMeta {
  pub fn new(index: i64, token_type: TokenTypesNames) -> TokenMeta {
    TokenMeta{ index, token_type }
  }
}


#[macro_export]
macro_rules! impl_base_token {
  ($struct_name:ident) => {
    impl $struct_name {
      pub fn repr(&self) -> String {
        format!("[token-type: '{:#?}' index({})]", self.meta.token_type, self.meta.index)
      }
    }
  }
}
