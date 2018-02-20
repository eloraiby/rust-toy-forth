use std::fs::File;
use std::io;
use std::fmt;
use std::env;
use std::collections::*;
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
    ip      : usize,
    code    : Vec<OpCode>,
    stack   : Vec<usize>,
    ret     : Vec<usize>,
    i_2_w   : Vec<Word>,
    i_2_n   : Vec<String>,
    n_2_i   : HashMap<String, usize>,

    except  : EvalResult,

    // REPL
    is_cm   : bool,                     // is it in compilation
    streams : Vec<StreamReader>,
    curr_sr : usize,                    // current stream reader index
    dead    : bool,
}

impl StreamReader {

    fn from_stdin() -> StreamReader {
        StreamReader {
            eos     : false,
            is      : InputStream::Cli(BufReader::new(io::stdin())),
            offset  : 0,        // where are we in the buffer
            buff    : Vec::new(),            
        }
    }

    fn from_file(file: File) -> StreamReader {
        StreamReader {
            eos     : false,
            is      : InputStream::File(BufReader::new(file)),
            offset  : 0,        // where are we in the buffer
            buff    : Vec::new(),            
        }
    }

    fn test_and_fill(&mut self) {
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

    fn read_char(&mut self) -> char {
        self.test_and_fill();
        if !self.is_eos() && self.offset < self.buff.len() {
            let ch = self.buff[self.offset];
            self.offset += 1;
            ch
        } else {
            '\0'
        }
    }

    fn peek_char(&mut self) -> char {
        self.test_and_fill();
        if !self.is_eos() {
            self.buff[self.offset]
        } else {
            '\0'
        }
    }

    fn is_eos(&self) -> bool { self.eos }   
}

mod core {
    use std::io;
    use std::io::Write;

    struct NativeEntry {
        name    : &'static str,
        is_macro: bool,
        f       : fn(&mut ::VM) -> ::EvalResult
    }

    pub const WID_CONSUME : usize = 0;

    fn define_word(vm: &mut ::VM) -> ::EvalResult {
        let token   = vm.read_token();
        vm.is_cm    = true;
        let wid     = vm.i_2_w.len();
        vm.i_2_w.push(::Word::Interp { is_macro: false, fip: vm.code.len() });
        vm.i_2_n.push(token.clone());
        vm.n_2_i.insert(token, wid);
        ::EvalResult::None
    }

    fn define_immediate(vm: &mut ::VM) -> ::EvalResult {
        let token   = vm.read_token();
        vm.is_cm    = false;
        let wid     = vm.i_2_w.len();
        vm.i_2_w.push(::Word::Interp { is_macro: true, fip: vm.code.len() });
        vm.i_2_n.push(token.clone());
        vm.n_2_i.insert(token, wid);
        ::EvalResult::None
    }

    fn ret(vm: &mut ::VM) -> ::EvalResult {
        let ip = vm.ret.pop().unwrap();
        vm.ip = ip;
        ::EvalResult::None
    }

    fn print_value_stack(vm: &mut ::VM) -> ::EvalResult {
        let stack = &vm.stack;
        print!("[ ");
        for v in stack {
            print!("{} ", v);
        }
        print!("]\n");
        io::stdout().flush().expect("stdout failed");
        ::EvalResult::None
    }

    fn print_return_stack(vm: &mut ::VM) -> ::EvalResult {
        for (i, r) in vm.ret.iter().enumerate() {
            println!("{} - {}", i, vm.i_2_n[*r]);
        }
        ::EvalResult::None
    }

    fn quit(vm: &mut ::VM) -> ::EvalResult {
        vm.dead = true;
        ::EvalResult::None
    }

    fn cm_set(vm: &mut ::VM) -> ::EvalResult {
        match vm.stack.pop() {
            Some(p) => {
                vm.is_cm = if p == 0 { false } else { true };
                ::EvalResult::None
                },
            None => ::EvalResult::StackUnderflow
        }
    }

    fn cm_get(vm: &mut ::VM) -> ::EvalResult {
        vm.stack.push(if vm.is_cm { 1 } else { 0 }) ;
        ::EvalResult::None
    }

    fn get_addr(vm: &mut ::VM) -> ::EvalResult {
        let tok = vm.read_token();
        match vm.n_2_i.contains_key(&tok) {
            true => { vm.stack.push(vm.n_2_i[&tok]); ::EvalResult::None },
            false => ::EvalResult::WordNotFound(tok)
        }
    }

    static CORE_ENTRIES : [NativeEntry; 10] = [
        NativeEntry { name : "consume", is_macro: true, f: ::VM::consume_token  }, 
        NativeEntry { name : "quit",    is_macro: true, f: quit                 }, 
        NativeEntry { name : ".s",      is_macro: true, f: print_value_stack    }, 
        NativeEntry { name : ".r",      is_macro: true, f: print_return_stack   }, 
        NativeEntry { name : ":",       is_macro: true, f: define_word          }, 
        NativeEntry { name : "!",       is_macro: true, f: define_immediate     }, 
        NativeEntry { name : "ret",     is_macro: true, f: ret                  }, 
        NativeEntry { name : "cm.set",  is_macro: true, f: cm_set               }, 
        NativeEntry { name : "cm.get",  is_macro: true, f: cm_get               }, 
        NativeEntry { name : "@",       is_macro: true, f: get_addr             }, 
    ];

    pub fn register(vm: &mut ::VM) {
        for ce in CORE_ENTRIES.iter() {
            vm.register_native(ce.name.to_string(), ce.is_macro, ce.f);
        }
    }
}


impl VM {
    fn new() -> VM {
        VM {
            ip      : 0,
            code    : Vec::new(),
            stack   : Vec::new(),
            ret     : Vec::new(),
            i_2_w   : Vec::new(),
            i_2_n   : Vec::new(),
            n_2_i   : HashMap::new(),

            except  : EvalResult::None,

            is_cm   : false,             // is in compilation mode
            streams : Vec::new(),
            curr_sr : 0,
            dead    : false,
        }
    }

    fn register_native(&mut self, name: String, is_macro: bool, f: fn(&mut VM) -> EvalResult) {
        let idx = self.i_2_w.len();
        self.i_2_w.push(Word::Native { is_macro : is_macro, f : f});
        self.i_2_n.push(name.clone());
        self.n_2_i.insert(name.clone(), idx);
        println!("registering {} @ {}", name, idx);
    }

    fn add_stream(&mut self, sr: StreamReader) {
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
            OpCode::Ret        => { let ip = self.ret.pop().unwrap(); self.ip = ip }
        }
    }

    fn consume_token(&mut self) -> EvalResult {
        let tok = self.read_token();
        match (self.is_cm, VM::is_number(&tok), self.n_2_i.contains_key(&tok)) {
            // handle number
            (true,  true, _    ) => { self.code.push(OpCode::PushUSize(tok.parse::<usize>().unwrap())); EvalResult::None },
            (false, true, _    ) => { self.stack.push(tok.parse::<usize>().unwrap()); EvalResult::None },

            // word
            (true,  _,    true ) => {
                let wid = self.n_2_i[&tok];
                match self.i_2_w[wid] {
                    Word::Interp { is_macro: true, fip }    => { self.ret.push(core::WID_CONSUME); self.ip = fip; EvalResult::None },
                    Word::Native { is_macro: true, f }      => f(self),
                    _                                       => { self.code.push(OpCode::Call(wid)); EvalResult::None }, 
                }
            },
            (false, _,    true ) => {
                let wid = self.n_2_i[&tok];
                match self.i_2_w[wid] {
                    Word::Interp { is_macro: _, fip }    => { self.ret.push(core::WID_CONSUME); self.ip = fip; EvalResult::None },
                    Word::Native { is_macro: _, f }      => f(self),
                }
            },
            // the word doesn't exist
            (_,     _,    false) => EvalResult::WordNotFound(tok),
        }
    }

    fn repl(&mut self) {
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
                EvalResult::StackUnderflow => println!("Error: Stack underflow"),
                EvalResult::WordNotFound(w) => println!("Error: Word {} not found!", w),
                EvalResult::ReturnStackUnderflow => println!("Error: return stack underflow")
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

    fn read_token(&mut self) -> String {
        let stream = 
            match self.streams[self.curr_sr].is_eos() {
                false => &mut self.streams[self.curr_sr],
                true  => { self.curr_sr += 1; &mut self.streams[self.curr_sr] }
            };
        VM::read_token_from_stream(stream)
    }
    
}


fn main() {
    let args = env::args();
    
    let mut vm  = VM::new();

    for (i, ref a) in args.enumerate().skip(1) {
        println!("stream {}: {}", i, a);
        match File::open(a) {
            Err(e) => panic!("File {} not found: {}", a, e),
            Ok(file) => vm.add_stream(StreamReader::from_file(file))
        }
    }
    core::register(&mut vm);
    vm.add_stream(StreamReader::from_stdin());
    vm.repl();
}
