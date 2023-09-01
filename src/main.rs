//extern crate serde;
//extern crate serde_yaml;

use std::env::{self};
use std::iter::*;
use std::fs::File;
use std::io::*;

mod helper;

  
pub fn new(argsv: Vec<String>) {
    if argsv.len() < 3 {
        panic!("\n Not enough arguments! Usage: \n \t plumber new <pipename>");
    }
    let plufile_name: String = format!("{}.plu.yaml", &argsv[2]);
    println!("    ~> New pipe: {}", plufile_name);
    let mut plufile = File::create(plufile_name).expect("Error encountered while creating file!");
    plufile.write_all(b"do: { \n \t echo hello world!\n }").expect("Error while writing to file");
}

pub fn argparse(argsv: Vec<String>, pos: usize, item: String) -> bool {
    // Parse arguments
    if argsv.len() > 1 && argsv[pos] == item {
        return true;
    } else {
        return false;
    }
}

pub fn cli() {
    // Main cli function
    let args: Vec<String> = env::args().collect(); // Argument collection
    // Parsing
    if argparse(args.clone(), 1, "new".to_string()) {
        new(args); // Create new plufile
    } else if argparse(args.clone(), 1, "run".to_string()) {
        let _ = helper::run(); // Run plufile
    } else if (argparse(args.clone(), 1,"help".to_string())) || 
              (argparse(args.clone(), 1, "--help".to_string())) ||
              (argparse(args.clone(), 1, "-h".to_string())) {
        let _ = helper::help(); //help
    } else {
        println!("Invalid Argument");
    }
}

fn main() {
    cli();
}
