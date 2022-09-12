use std::io::{self, BufReader, BufRead};

pub struct LogCompare<R : io::Read> {
    log_file : BufReader<R>,
    line_number : usize,
}

impl <R : io::Read> LogCompare<R> {
    pub fn new(expect_file : R) -> Self {
        Self {
            log_file : BufReader::new(expect_file),
            line_number : 0,
        }
    }

    pub fn line(&mut self, line : &str) -> bool {
        let mut buf  = String::new();
        let result = self.log_file.read_line(&mut buf);
        match result {
            // EOF
            Ok(0) => false,
            Ok(_) => {
                let chomped = &buf[..buf.len()-1];
                chomped.eq(line)
            }
            Err(e) => {
                println!("log file read error {:?}", e);
                false
            }
        }
    }

    pub fn line_number(&self) -> usize {
        self.line_number
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
        let mut compare = LogCompare::new(b);
        assert!(compare.line("Log Start"));
        assert!(compare.line("f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8000: 78       SEI"));
        assert!(compare.line("f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8001: D8       CLD"));
    }
}