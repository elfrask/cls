use crate::cls::lib::structs::tokens::simples::{enums::SimpleTokensEnum, name_token::NameToken, number_token::NumberToken};

fn es_identificador_valido(s: &str) -> bool {
    !s.is_empty() && 
    s.chars().next().unwrap().is_alphabetic() &&  // Debe empezar con letra
    s.chars().all(|c| c.is_alphanumeric() || c == '_')  // Solo letras, números, _
}

pub fn passToken(index: i64, value: &str) -> SimpleTokensEnum {

  if es_identificador_valido(value) {
    return SimpleTokensEnum::Name(NameToken::new(index, &value.to_string().as_str()));
  }

  if let Ok(v) = value.parse::<i64>() {
    return SimpleTokensEnum::Number(NumberToken::newInt(index, v));
  }

  if let Ok(v) = value.parse::<f64>() {
    return SimpleTokensEnum::Number(NumberToken::newFloat(index, v));
  }

  // unreachable!();
  panic!("Token inválido: '{}'", value);
}

pub fn debug_tokenizer(tokenList: &Vec<Vec<SimpleTokensEnum>>) {

  println!("longitud de tokens: {}", tokenList.len());


  println!("[");
  for line_tokens in tokenList {
    println!("  [");
    for token in line_tokens {
      print!("    ");
      println!("{}", token.repr());
    };
    println!("  ]");
  };
  println!("]");
}