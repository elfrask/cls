use std::rc::Rc;

use crate::cls::lib::structs::tokens::simples::enums::SimpleTokensEnum;



pub struct Script {
  pub path: String,
  pub code_raw: String,
  pub pid: i32,
  pub tokens: Vec<Vec<SimpleTokensEnum>>
}


impl Script {
  pub fn new(path: &str, code: &str, pid: i32) -> Script {
    Script{ 
      path: path.to_string(),
      code_raw: code.to_string(),
      pid,
      tokens: Vec::new()
    }
  }
}