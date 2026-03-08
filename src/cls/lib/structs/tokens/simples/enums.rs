use crate::cls::lib::structs::tokens::simples::{number_token, operator_token, string_token};

use super::name_token;
// Enums Simples
pub enum SimpleTokensEnum {
  Name(name_token::NameToken),
  Number(number_token::NumberToken),
  Operator(operator_token::OperatorToken),
  String(string_token::StringToken)
}


