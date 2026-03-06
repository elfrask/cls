

pub struct Script {
  pub path: &str,
  pub code: &str,
  pub pid: i32,
}


impl Script {
  pub fn new(path: &str, code: &str) -> Script {
    Script{ 
      path,
      code,
    }
  }


}