use super::parser::tokenizer;

#[allow(COPY, CLONE)]
pub fn run_file(_path: &str, _code: &str) -> bool {
  

  let mut token_parsed: tokenizer::Tokenizador = tokenizer::Tokenizador::new(_path);
  token_parsed.parse(_code);

  print!("Compilado!");

  return true
}

