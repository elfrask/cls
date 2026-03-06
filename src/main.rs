#![allow(warnings)]
use std::env::args;
mod cls;
use std::{error, fs};
// use std::env::Args;
// use core::iter::traits::iterator::Iterator;

fn main() {
    let cli_args: Vec<String> = args().collect();

    if let Some(_path) = cli_args.get(1) {
        let code = fs::read_to_string(_path);

        match code {
            Ok(contenido) => {
                cls::cls::run_file(_path, &contenido);
            }
            Err(error) => {
                print!("File '{_path}' not found")
            }
        }
    };
}
