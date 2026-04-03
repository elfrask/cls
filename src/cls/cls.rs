use crate::cls::{environment::script::Script, lib::structs::tokens::lib::debug_tokenizer};

use super::parser::tokenizer;

#[allow(COPY, CLONE)]
pub fn run_file(_path: &str, _code: &str) -> bool {
  
  let mut script = Script::new(_path, _code, 1);

  tokenizer::Tokenizador::new(&mut script).parse();
  // token_parsed.parse();
  // token_parsed.parse();

  println!("Compilado!");
  debug_tokenizer(&script.tokens);

  return true
}

