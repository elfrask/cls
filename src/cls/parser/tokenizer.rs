use std::ptr::eq;
use std::rc::Rc;

use crate::cls::environment::script::Script;
use crate::cls::{consts::tokens::simple::{COMPUESTOS, DELIMITADORES, DELIMITADORES_STRINGS, OPERADORES, SIMBOLOS}, lib::structs::tokens::{lib::passToken, simples::{enums::SimpleTokensEnum, operator_token::OperatorToken, string_token::StringToken, symbol_token::SymbolToken}}};
// use crate::environment::script::Script, lib::structs::tokens::{lib::passToken, simples::{enums::SimpleTokensEnum, operator_token::OperatorToken, string_token::StringToken, symbol_token::SymbolToken}}}

// use crate::
pub const LEX_VERSION: &str = "0.1.0";

enum States {
  Main,
  String,
  CommentLine,
  CommentMultiLine
}


// pub struct Tokenizador {
pub struct Tokenizador<'a> {
  // pub script: Script,
  pub script: &'a mut Script,
  
  // pub result: Vec<Vec<SimpleTokensEnum>>,
  stack_line: Vec<SimpleTokensEnum>,
  stack: String,
  state: States,
  pub ok: bool,
  
}

// impl Tokenizador {
impl<'a> Tokenizador<'a> {
  // pub fn new(script: Script) -> Self {
  pub fn new(script: &'a mut Script) -> Self {
    Tokenizador { 
      // result: Vec::new(),
      stack_line: Vec::new(),
      script: script,
      state: States::Main,
      stack: "".to_string(),
      ok: true,
    }
  }
  fn next_stack(&mut self, index: i64) {
    if !self.stack.is_empty() {
      self.stack_line.push(passToken(index, &self.stack));
      self.stack = String::new()
    }
  }
  fn next_line(&mut self, index: i64) {
    self.next_stack(index);
    if !self.stack_line.is_empty() {
      self.script.tokens.push(std::mem::take(&mut self.stack_line));
    }
  }
  pub fn parse(&mut self) -> &Vec<Vec<SimpleTokensEnum>> {
    
    let code: &str = &self.script.code_raw.clone();
    let chars = (code).chars().enumerate();
    // let mut iter = chars.iter().enumerate().peekable();
   
    // self.result = Vec::new();
    self.stack_line = Vec::new();
    self.stack = String::new();
    self.state = States::Main;
    self.ok = true;

    let mut cursor: i64 = -1;


    // while let Some((index, character)) = iter.peek()
    for (_index, (_, _character)) in chars.enumerate() {
      // process each char
      let character = &&_character;
      let index = (_index) as i64;
      cursor = index;
      let match_char = character.to_string();
      // println!("llego {}: '{}'", index.clone(), _character);
      
      match self.state {
        States::Main => {

          // Delimitadores
          if **character == ';' {
            self.next_line(index);
            continue;
          } 

          // Espacios vacíos

          if DELIMITADORES.contains(&match_char.as_str()) {
            self.next_stack(index);   
            continue;
          }

          // Operadores y Operadores compuestos

          if OPERADORES.contains(&match_char.as_str()) {
            self.next_stack(index);
            
            if **character == '#' {
              self.state = States::CommentLine;
              continue;
            }

            match self.stack_line.last_mut() {
              Some(SimpleTokensEnum::Operator(_ope)) => {
                if (index - _ope.operator.len() as i64) == _ope.meta.index  {
                  let compuesto = format!("{}{}", _ope.operator, character);
                  if COMPUESTOS.contains(&compuesto.as_str()) {
                    _ope.push_operator(**character);
                    continue;
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
            continue;
          }

          // Símbolos jerarquizados

          if SIMBOLOS.contains(&match_char.as_str()) {
            self.next_stack(index);
            self.stack_line.push(
              SimpleTokensEnum::Symbol(
                SymbolToken::new(index, **character)
              )
            );
            continue;
          }

          // Generar Cadena de texto
          
          if DELIMITADORES_STRINGS.contains(&match_char.as_str()) {
            // self.next_stack(index);
            // let mut _format = String::new();

            // if let Some(SimpleTokensEnum::Name(v)) = self.stack_line.last() {
            //   _format = v.name.clone();
            //   self.stack_line.pop();
            // }

            self.stack_line.push(
              SimpleTokensEnum::String(
                StringToken::new(index, **character, Some(self.stack.clone()))
              )
            );
            self.stack = String::new();
            self.state = States::String;
            continue;
          }

          // Adición por descarte al stack 

          self.stack.push(**character);
        } 
        States::CommentLine => {
          if **character == '\n' {
            self.next_line(index);
            self.state = States::Main;
          }
          continue;
        }
        States::String => {
          if let Some(SimpleTokensEnum::String( v)) = self.stack_line.last_mut() {
            if **character == v.delimiter {
              self.state = States::Main;
              continue;  
            };

            v.push_char(**character);
          }
        }
        _ => {} 
      }


    };

    match self.state {
      States::Main | 
      States::CommentLine |
      States::CommentMultiLine
      => {
        self.next_line(cursor as i64);
      }
      _ => {
        self.ok = false;
        panic!("Hay scopes abierto, por favor ciérralos")
      } 
    }

    // println!("longitud de tokens: {}", self.result.len());
    // self.script.tokens = self.result;
    return &self.script.tokens;
  } 
}