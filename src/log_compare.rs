use std::fs::File;
use std::io::{self, BufReader};

struct LogCompare<T> {
    file : BufReader<T>,

}

impl <R : io::Read> LogCompare<R> {
    pub fn new(expectFile : R) -> Self {
        Self {
            file : BufReader::new(expectFile)
        }
    }

    pub fn line(line : String) -> bool {
        true
    }
}

#[cfg(test)]
mod LogCompareTest {
    use super::*;

    #[test]
    pub fn line_test() {
        let b = "Log Start
f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8000: 78       SEI
f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8001: D8       CLD
".as_bytes();
        
        let f = LogCompare::new(b);
    }
}