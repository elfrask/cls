// use crate::
pub const LEX_VERSION: &str = "0.1.0";

enum States {
  Main
}


pub struct Tokenizador {
  result: Vec<Vec<String>>,
  pathFile: String,
  state: States,
  stack: String,
}

impl Tokenizador {
  pub fn new(pathFile: &str) -> Self {
    Tokenizador { 
      result: Vec::new(),
      pathFile: pathFile.to_string(),
      state: States::Main,
      stack: "".to_string(),
    }
  }
  pub fn parse(&mut self, code: &str) {
    let mut output: Vec<Vec<String>> = Vec::new();

    self.state = States::Main;

    for char in code.chars() {
      // process each char
      
    }

  } 
}