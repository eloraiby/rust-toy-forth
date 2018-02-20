use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Write};

pub enum InputStream {
    File(io::BufReader<File>),
    Cli(io::BufReader<io::Stdin>)
}

pub struct StreamReader {
    eos     : bool,
    is      : InputStream,
    offset  : usize,        // where are we in the buffer
    buff    : Vec<char>,
}

impl StreamReader {

    pub fn from_stdin() -> StreamReader {
        StreamReader {
            eos     : false,
            is      : InputStream::Cli(BufReader::new(io::stdin())),
            offset  : 0,        // where are we in the buffer
            buff    : Vec::new(),            
        }
    }

    pub fn from_file(file: File) -> StreamReader {
        StreamReader {
            eos     : false,
            is      : InputStream::File(BufReader::new(file)),
            offset  : 0,        // where are we in the buffer
            buff    : Vec::new(),            
        }
    }

    pub fn test_and_fill(&mut self) {
        if !self.is_eos() && self.offset >= self.buff.len() {
            let mut buff = String::new();
                
            let (res, is_eos) =
                match self.is {
                    InputStream::File(ref mut f)    => (f.read_line(&mut buff), buff.len() == 0 ),
                    InputStream::Cli(ref mut c)     => {
                        print!("> ");
                        io::stdout().flush().expect("stdout couldn't flush");
                        (c.read_line(&mut buff), false) 
                    }
                };

            match (res, is_eos)  {
                (_, true) | (Err(_), _) => {
                    self.eos        = true;
                    self.offset     = 0;
                    self.buff.clear();
                } ,
                (Ok(_), false) => {
                    self.offset     = 0;
                    self.buff       = buff.chars().collect();
                }
            }
        }
    }

    pub fn read_char(&mut self) -> char {
        self.test_and_fill();
        if !self.is_eos() && self.offset < self.buff.len() {
            let ch = self.buff[self.offset];
            self.offset += 1;
            ch
        } else {
            '\0'
        }
    }

    pub fn peek_char(&mut self) -> char {
        self.test_and_fill();
        if !self.is_eos() {
            self.buff[self.offset]
        } else {
            '\0'
        }
    }

    pub fn is_eos(&self) -> bool { self.eos }   
}

