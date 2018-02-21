use std::fmt;
use std::collections::*;
use stream::StreamReader;

pub enum EvalResult {
    None,
    StackUnderflow,
    ReturnStackUnderflow,
    WordNotFound(String),
}

#[derive(Clone, Copy, Debug)]
pub enum OpCode {
    PushUSize(usize),
    Jmp(usize),
    Call(usize),
    Cond(usize),    // jump when the value on top of the stack is true
    Ret,
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OpCode::PushUSize(u)    => write!(f, "PushUSize({})", u),
            OpCode::Jmp(u)          => write!(f, "Jmp({})", u),
            OpCode::Call(u)         => write!(f, "Call({})", u),
            OpCode::Cond(u)         => write!(f, "Cond({})", u),
            OpCode::Ret             => write!(f, "Ret"),
        }
    }
}

pub enum Word {
    Native { is_macro: bool, f: fn(&mut VM) -> EvalResult },
    Interp { is_macro: bool, fip: usize },
}


pub struct VM {
    pub ip      : usize,
    pub code    : Vec<OpCode>,
    pub stack   : Vec<usize>,
    pub ret     : Vec<usize>,
    pub i_2_w   : Vec<Word>,
    pub i_2_n   : Vec<String>,
    pub n_2_i   : HashMap<String, usize>,

    // REPL
    pub is_cm   : bool,                     // is it in compilation
    pub streams : Vec<StreamReader>,
    pub curr_sr : usize,                    // current stream reader index
    pub dead    : bool,
}

impl VM {
    pub fn new() -> VM {
        VM {
            ip      : 0,
            code    : Vec::new(),
            stack   : Vec::new(),
            ret     : Vec::new(),
            i_2_w   : Vec::new(),
            i_2_n   : Vec::new(),
            n_2_i   : HashMap::new(),

            is_cm   : false,             // is in compilation mode
            streams : Vec::new(),
            curr_sr : 0,
            dead    : false,
        }
    }

    pub fn register_native(&mut self, name: String, is_macro: bool, f: fn(&mut VM) -> EvalResult) {
        let idx = self.i_2_w.len();
        self.i_2_w.push(Word::Native { is_macro : is_macro, f : f});
        self.i_2_n.push(name.clone());
        self.n_2_i.insert(name.clone(), idx);
        println!("registering {} @ {}", name, idx);
    }

    pub fn add_stream(&mut self, sr: StreamReader) {
        self.streams.push(sr)
    }

    fn next_word(&mut self) {
        let word = self.code[self.ip];
        println!("{} - {}", self.ip, word);
        match word {
            OpCode::Cond(ip)   => { let v = self.stack.pop().unwrap(); if v != 0 { self.ip = ip } },
            OpCode::Jmp(ip)    => { self.ip = ip },
            OpCode::Call(w)   => {
                match self.i_2_w[w] {
                    Word::Interp { is_macro: _, fip }   => { self.ret.push(self.ip + 1); self.ip = fip },
                    Word::Native { is_macro: _, f }     => { f(self); self.ip += 1 }
                    }
                },
            OpCode::PushUSize(u)   => { self.stack.push(u); self.ip += 1 },
            OpCode::Ret        => {
                match self.ret.pop() {
                    Some(ip) => { self.ip = ip },
                    None => panic!("TODO")
                }
            }
        }
    }

    fn consume_token(&mut self) -> EvalResult {
        let tok = self.read_token();
        println!("reading {} - CM {}", tok.clone(), self.is_cm);
        match (self.is_cm, VM::is_number(&tok), self.n_2_i.contains_key(&tok)) {
            // handle number
            (true,  true, _    ) => { self.code.push(OpCode::PushUSize(tok.parse::<usize>().unwrap())); EvalResult::None },
            (false, true, _    ) => { self.stack.push(tok.parse::<usize>().unwrap()); EvalResult::None },

            // word
            (true,  _,    true ) => {
                let wid = self.n_2_i[&tok];
                match self.i_2_w[wid] {
                    Word::Interp { is_macro: true, fip } => { self.ip = fip; EvalResult::None },
                    Word::Native { is_macro: true, f }   => f(self),
                    _                                    => { self.code.push(OpCode::Call(wid)); EvalResult::None }, 
                }
            },
            (false, _,    true ) => {
                let wid = self.n_2_i[&tok];
                match self.i_2_w[wid] {
                    Word::Interp { is_macro: _, fip }    => { self.ip = fip; EvalResult::None },
                    Word::Native { is_macro: _, f }      => f(self),
                }
            },

            // the word doesn't exist
            (_,     _,    false) =>
                EvalResult::WordNotFound(tok),
        }
    }

    pub fn repl(&mut self) {
        loop {
            let res = self.consume_token();
            match res {
                EvalResult::None =>
                    loop {
                        match (self.ret.len(), self.dead) {
                            (0, _) | (_, true) => break,
                            _ => self.next_word()
                        }
                    },
                EvalResult::StackUnderflow          => println!("Error: Stack underflow"),
                EvalResult::WordNotFound(w)         => println!("Error: Word {} not found!", w),
                EvalResult::ReturnStackUnderflow    => println!("Error: return stack underflow")
            }

            if self.dead { break; }
        }
    }

    fn is_digit(c: char) -> bool {
        if c >= '0' && c <= '9' { true }
        else { false }
    }

    fn is_number(token: &String) -> bool {
        for c in token.chars() {
            if !VM::is_digit(c) { return false }
        }
        true
    }
/*
    fn read_token_from_stream(sr: &mut StreamReader) -> String {
        let mut token = String::new();
        while !sr.is_eos() {
            match sr.read_char() {
                ' ' | '\n' | '\t' => break,
                ch => token.push(ch)
            }
        }

        token
    }

    pub fn read_token(&mut self) -> String {
        let stream = 
            match self.streams[self.curr_sr].is_eos() {
                false => &mut self.streams[self.curr_sr],
                true  => { self.curr_sr += 1; &mut self.streams[self.curr_sr] }
            };
        VM::read_token_from_stream(stream)
    }
*/
    pub fn read_stream_char(&mut self) -> char {
        let stream = 
            match self.streams[self.curr_sr].is_eos() {
                false => &mut self.streams[self.curr_sr],
                true  => { self.curr_sr += 1; &mut self.streams[self.curr_sr] }
            };
        stream.read_char()
    }

    pub fn read_token(&mut self) -> String {
        let mut token = String::new();
        loop {

            match self.read_stream_char() {
                ' ' | '\n' | '\t' => break,
                ch => token.push(ch)
            }
        }
        token       
    }

    pub fn read_char(vm: &mut VM) -> EvalResult {
        let stream = 
            match vm.streams[vm.curr_sr].is_eos() {
                false => &mut vm.streams[vm.curr_sr],
                true  => { vm.curr_sr += 1; &mut vm.streams[vm.curr_sr] }
            };
        vm.stack.push(stream.read_char() as usize);
        EvalResult::None
    }

    pub fn peek_char(vm: &mut VM) -> EvalResult {
        let stream = 
            match vm.streams[vm.curr_sr].is_eos() {
                false => &mut vm.streams[vm.curr_sr],
                true  => { vm.curr_sr += 1; &mut vm.streams[vm.curr_sr] }
            };
        vm.stack.push(stream.peek_char() as usize);
        EvalResult::None
    }

}
