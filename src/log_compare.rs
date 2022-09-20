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

    pub fn line(&mut self, line : &str) -> (bool, String) {
        let mut buf  = String::new();
        let result = self.log_file.read_line(&mut buf);
        let expect = buf[..buf.len()-1].to_string();
        let result = match result {
            // EOF
            Ok(0) => false,
            Ok(_) => {
                expect.eq(line)
            }
            Err(e) => {
                println!("log file read error {:?}", e);
                false
            }
        };
        (result, expect)
    }

    pub fn line_number(&self) -> usize {
        self.line_number
    }

    pub fn test_line(&mut self, line: &str) {
        let (result, expect) = self.line(&line);
        if !result {
            println!("wrong : line : {:}", self.line_number()+1);
            println!("wrong : expect : [{:}]", expect);
            println!("wrong : actual : [{:}]", line);
            panic!("fail to match")
        }
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
        assert!(compare.line("Log Start").0);
        assert!(compare.line("f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8000: 78       SEI").0);
        assert!(compare.line("f1      A:00 X:00 Y:00 S:FD P:nvubdIzc  $8001: D8       CLD").0);
    }
}