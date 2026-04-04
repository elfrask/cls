use crate::cls::lib::structs::tokens::simples::{name_token, node_token, number_token, operator_token, string_token, symbol_token};
use crate::cls::lib::structs::tokens::meta::BaseToken;
// Enums Simples

macro_rules! impl_repr {
    ($enum_name:ident, $($variant:ident),+) => {
        impl $enum_name {
            pub fn repr(&self, prefix: &str) -> String {
                match self {
                    $( $enum_name::$variant(token) => token.repr(prefix), )+
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
  Symbol(symbol_token::SymbolToken),
  Node(node_token::NodeToken),
}


impl_repr!(SimpleTokensEnum, Name, Number, Operator, String, Symbol, Node);
