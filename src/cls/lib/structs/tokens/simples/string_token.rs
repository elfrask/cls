use crate::{cls::lib::structs::tokens::{meta::{BaseToken, TokenMeta}, types::TokenTypesNames}};

pub struct StringToken {
  pub meta: TokenMeta,
  pub delimiter: char,
  pub format: String,
  pub string: String,
}

// impl_base_token!(StringToken);

impl StringToken {
  pub fn new(index: i64, delimiter: char, format: Option<String>) -> Self {
    let mut _format: String = "".to_string(); 
    
    if let Some(v) = format {
      _format = v;  
    };
    
    StringToken {
      meta: TokenMeta::new(index, TokenTypesNames::String),
      delimiter,
      string: "".to_string(),
      format: _format
    }
  }
  pub fn push_char(&mut self, character: char) {
    self.string.push(character);
  }
  pub fn push_string(&mut self, string: &str) {
    self.string.push_str(string);
    
  }

  
  
}

impl BaseToken for StringToken {
  fn repr(&self, prefix: &str) -> String {
    format!("{}: '{}'", self.meta, self.string)
  }
}
