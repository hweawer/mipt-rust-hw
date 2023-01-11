#![forbid(unsafe_code)]

use std::fs::ReadDir;
use std::sync::{Arc, Mutex};
use std::{
    fs,
    fs::File,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    sync::mpsc::{self, Sender},
    thread,
};

use rayon::prelude::*;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    pub path: PathBuf,
    pub line: String,
    pub line_number: usize,
}

#[derive(Debug)]
pub struct Error {
    pub path: PathBuf,
    pub error: io::Error,
}

pub enum Event {
    Match(Match),
    Error(Error),
}

pub fn run<P: AsRef<Path>>(path: P, pattern: &str) -> Vec<Event> {
    if !path.as_ref().is_dir() {
        let mut res: Vec<Event> = Vec::new();
        let file = match File::open(path.as_ref()) {
            Ok(f) => f,
            Err(err) => {
                res.push(Event::Error(Error {
                    path: PathBuf::from(path.as_ref()),
                    error: err,
                }));
                return res;
            }
        };
        let reader = BufReader::new(file);

        for (line_number, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(err) => {
                    res.push(Event::Error(Error {
                        path: PathBuf::from(path.as_ref()),
                        error: err,
                    }));
                    return res;
                }
            };
            if line.contains(pattern) {
                res.push(Event::Match(Match {
                    path: PathBuf::from(path.as_ref()),
                    line,
                    line_number: line_number + 1,
                }));
            }
        }
        res
    } else {
        let read_dir = match fs::read_dir(path.as_ref()) {
            Ok(r) => r,
            Err(err) => {
                let mut res = Vec::with_capacity(1);
                res.push(Event::Error(Error {
                    path: PathBuf::from(path.as_ref()),
                    error: err,
                }));
                return res;
            }
        };
        read_dir
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>()
            .par_iter()
            .map(|path| run(path, pattern))
            .flatten()
            .collect()
    }
}
