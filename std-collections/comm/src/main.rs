#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let args = env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3);
    let mut set = HashSet::new();
    let mut res = HashSet::new();
    let mut file = File::open(&args[1]).unwrap();
    let mut reader = BufReader::new(file);
    for line in reader.lines() {
        set.insert(line.unwrap());
    }
    file = File::open(&args[2]).unwrap();
    reader = BufReader::new(file);
    for line in reader.lines() {
        let s = line.unwrap();
        if set.contains(&s) {
            res.insert(s);
        }
    }
    for line in res {
        println!("{}", line);
    }
}
