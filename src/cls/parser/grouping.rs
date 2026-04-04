use crate::cls::{environment::script::Script, lib::structs::tokens::simples::enums::SimpleTokensEnum};




pub struct Grouping<'a> {
  pub script: &'a mut Script,
  
  stack: Vec<Vec<Vec<SimpleTokensEnum>>>
}


impl<'a> Grouping<'a> {
  fn new(script: &'a mut Script) -> Grouping {
    Grouping{ 
      script,
      stack: Vec::new()
    }
  }

  pub fn parse_grouping(&mut self) -> &Vec<Vec<SimpleTokensEnum>> {
    
    let raw = &mut self.script.tokens;
    
    for (index) in raw {
      


    }
    

    return &self.script.tokens;
  }
}