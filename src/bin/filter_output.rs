use std::io::{BufReader, BufRead, LineWriter, Write};
use std::fs::File;

pub fn main() {
    let reader = BufReader::new(File::open("output.txt").expect("input file open"));
    let mut writer = LineWriter::new(File::create("filtered.txt").expect("output file open"));
    for line in reader.lines() {
        let line = line.expect("read line");
        if !line.contains("does not contain a target predicate") {
            writer.write_all(format!("{line}\n").as_bytes()).expect("write line");
        }
    }
}