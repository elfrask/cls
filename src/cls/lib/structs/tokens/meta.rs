use core::fmt;
use std::fmt::write;

use crate::cls::lib::structs::tokens::types::TokenTypesNames;

pub trait BaseToken {
  fn repr(&self, prefix: &str) -> String;
  // fn repr(&self) -> &'static str;
}


#[derive(Debug)]
pub struct TokenMeta {
  pub index: i64,
  pub token_type: TokenTypesNames,
}

impl TokenMeta {
  pub fn new(index: i64, token_type: TokenTypesNames) -> TokenMeta {
    TokenMeta{ index, token_type }
  }
  pub fn to_string(&self) -> String {
    format!("[token-type: '{:#?}' index({})]", self.token_type, self.index)
  }
}

impl std::fmt::Display for TokenMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.to_string())
    }

}

pub fn reprDebug(meta: &TokenMeta) -> String {
  format!("[token-type: '{:#?}' index({})]", meta.token_type, meta.index)
}


// #[macro_export]
// macro_rules! impl_base_token {
//   ($struct_name:ident) => {
//     impl $struct_name {
//       pub fn repr(&self) -> String {
//         return format!("{}", self.meta)
//       }
//     }
//   }
// }
