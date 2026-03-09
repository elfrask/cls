use crate::cls::lib::structs::tokens::simples::{number_token, operator_token, string_token, symbol_token, name_token};
use crate::cls::lib::structs::tokens::meta::BaseToken;
// Enums Simples

macro_rules! impl_repr {
    ($enum_name:ident, $($variant:ident),+) => {
        impl $enum_name {
            pub fn repr(&self) -> String {
                match self {
                    $( $enum_name::$variant(token) => token.repr(), )+
                }
            }
        }
    };
}

pub enum SimpleTokensEnum {
  Name(name_token::NameToken),
  Number(number_token::NumberToken),
  Operator(operator_token::OperatorToken),
  String(string_token::StringToken),
  Symbol(symbol_token::SymbolToken)
}


impl_repr!(SimpleTokensEnum, Name, Number, Operator, String, Symbol);
