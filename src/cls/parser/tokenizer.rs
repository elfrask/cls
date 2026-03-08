use crate::cls::{consts::tokens::simple::{COMPUESTOS, DELIMITADORES}, lib::structs::tokens::{lib::passToken, simples::{enums::SimpleTokensEnum, operator_token::OperatorToken}}};

// use crate::
pub const LEX_VERSION: &str = "0.1.0";

enum States {
  Main
}


pub struct Tokenizador {
  pathFile: String,
  
  result: Vec<Vec<SimpleTokensEnum>>,
  stack_line: Vec<SimpleTokensEnum>,
  stack: String,
  state: States,
  
}

impl Tokenizador {
  pub fn new(pathFile: &str) -> Self {
    Tokenizador { 
      result: Vec::new(),
      stack_line: Vec::new(),
      pathFile: pathFile.to_string(),
      state: States::Main,
      stack: "".to_string(),
    }
  }
  fn next_stack(&mut self, index: i64) {
    if !self.stack.is_empty() {
      self.stack_line.push(passToken(index, &self.stack));
      self.stack = String::new()
    }
  }
  fn next_line(&mut self, index: i64) {
    if !self.stack.is_empty() {
      self.stack_line.push(passToken(index, &self.stack));
      self.stack = String::new()
    }
  }
  pub fn parse(&mut self, code: &str) -> &Vec<Vec<SimpleTokensEnum>> {
    
    let chars: Vec<char> = code.chars().collect();
    let mut iter = chars.iter().enumerate().peekable();
   
    self.result = Vec::new();
    self.stack_line = Vec::new();
    self.stack = String::new();
    self.state = States::Main;
    


    while let Some((index, character)) = iter.peek() {
      // process each char
      let index = *index as i64;
      let match_char = character.to_string();
      
      match self.state {
        States::Main => {

          if DELIMITADORES.contains(&match_char.as_str()) {
            self.next_stack(index);

            match self.stack_line.last_mut() {
              Some(SimpleTokensEnum::Operator(_ope)) => {
                if (index - _ope.operator.len() as i64) == _ope.meta.index  {
                  let compuesto = format!("{}{}", _ope.operator, character);
                  if COMPUESTOS.contains(&compuesto.as_str()) {
                    _ope.push_operator(**character);
                  }
                }
              }
              _ => {}
            };
            self.stack_line.push(
              SimpleTokensEnum::Operator(
                OperatorToken::new(index, **character)
              )
            );
          }

        } 
        _ => {

        } 
      }

    };

    return &self.result;
  } 
}