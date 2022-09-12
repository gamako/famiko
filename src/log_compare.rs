use std::io::{self, BufReader, BufRead};

pub struct log_compare<R : io::Read> {
    log_file : BufReader<R>,

}

impl <R : io::Read> log_compare<R> {
    pub fn new(expectFile : R) -> Self {
        Self {
            log_file : BufReader::new(expectFile)
        }
    }

    pub fn line(&mut self, line : &str) -> bool {
        let mut buf  = String::new();
        self.log_file.read_line(&mut buf);
        let chomped = &buf[..buf.len()-1];
        
        chomped.eq(line)
    }
}

#[cfg(test)]
mod log_compare_test {
    use super::*;

    #[test]
    pub fn line_test() {
        let b = "Log Start
f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8000: 78       SEI
f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8001: D8       CLD
".as_bytes();
        let mut compare = log_compare::new(b);
        assert!(compare.line("Log Start"));
        assert!(compare.line("f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8000: 78       SEI"));
        assert!(compare.line("f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8001: D8       CLD"));
    }
}