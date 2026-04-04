
use crate::cls::lib::structs::tokens::{meta::{BaseToken, TokenMeta}, simples::enums::SimpleTokensEnum, types::TokenTypesNames};

#[derive(Debug)]
pub enum NodeTokenTypes {
  Parentheses, // ()
  Brackets, // []
  Keys // {}
}

pub struct NodeToken {
  pub meta: TokenMeta,
  pub content: Vec<Vec<SimpleTokensEnum>>,
  pub nodeType: NodeTokenTypes
}


// impl_base_token!(NodeToken);

impl NodeToken {
  pub fn new(index: i64, nodeType: NodeTokenTypes) -> NodeToken {
    NodeToken {
      meta: TokenMeta::new(index, TokenTypesNames::Node),
      // name: name.to_string(),
      content: Vec::new(),
      nodeType
    }
  }

  pub fn getExpression(&self) -> &[SimpleTokensEnum] {
    
    if (self.content.is_empty()) {
      return &[];
    }

    return &self.content[0];
  }
}


impl BaseToken for NodeToken  {
  fn repr(&self, prefix: &str) -> String {
    let mut output = "".to_string();
    let mut newPrefix = prefix.to_string();

    let mut openLine = prefix.to_string();
    openLine.push_str("  [\n");
    let mut closeLine = prefix.to_string();
    openLine.push_str("  ]\n");
    

    newPrefix.push_str("    ");

    for e in &self.content {
      output.push_str(&openLine);
      for ee in e {
        let result = ee.repr(&newPrefix);
        let mut line = "    ".to_string();
        line.push_str(&prefix);
        line.push_str(&result);
        line.push_str("\n");
        output.push_str(&line)
      };
      output.push_str(&closeLine);

    };

    return format!("{}: '{:?}': [
{}
    {}]", self.meta, self.nodeType, output, prefix);
  }
}