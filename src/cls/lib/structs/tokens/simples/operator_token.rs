use crate::{cls::lib::structs::tokens::{meta::TokenMeta, types::TokenTypesNames}, impl_base_token};

pub struct OperatorToken {
  pub meta: TokenMeta,
  pub operator: String,
}

impl_base_token!(OperatorToken);

impl OperatorToken {
  pub fn new(index: i64, operator: char) -> OperatorToken {
    OperatorToken {
      meta: TokenMeta::new(index, TokenTypesNames::Operator),
      operator: operator.to_string()
    }
  }
  pub fn push_operator(&mut self, post_operator: char) {
    self.operator.push(post_operator);
    // self.operator = format!("{}{}", self.operator, post_operator)
  }
}
