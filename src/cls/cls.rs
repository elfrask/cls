use crate::cls::lib::structs::tokens::lib::debug_tokenizer;

use super::parser::tokenizer;

#[allow(COPY, CLONE)]
pub fn run_file(_path: &str, _code: &str) -> bool {
  

  let mut token_parsed: tokenizer::Tokenizador = tokenizer::Tokenizador::new(_path);
  token_parsed.parse(_code);

  println!("Compilado!");
  debug_tokenizer(&token_parsed.result);

  return true
}

