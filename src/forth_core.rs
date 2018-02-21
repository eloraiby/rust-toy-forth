use std::io;
use std::io::{Write};
use vm::*;

struct NativeEntry {
    name    : &'static str,
    is_macro: bool,
    f       : fn(&mut VM) -> EvalResult
}

fn define_word(vm: &mut VM) -> EvalResult {
    let token   = vm.read_token();
    match token {
        Some(token) => {
            vm.is_cm    = true;
            let wid     = vm.i_2_w.len();
            vm.i_2_w.push(Word::Interp { is_macro: false, fip: vm.code.len() });
            vm.i_2_n.push(token.clone());
            vm.n_2_i.insert(token, wid);
            EvalResult::None
        },
        None => EvalResult::EmptyToken
    }
}

fn define_immediate(vm: &mut VM) -> EvalResult {
    let token   = vm.read_token();
    match token {
        Some(token) => {
            vm.is_cm    = true;
            let wid     = vm.i_2_w.len();
            vm.i_2_w.push(Word::Interp { is_macro: true, fip: vm.code.len() });
            vm.i_2_n.push(token.clone());
            vm.n_2_i.insert(token, wid);
            EvalResult::None
        },
        None => EvalResult::EmptyToken,
    }
}

fn eofn(vm: &mut VM) -> EvalResult {
    vm.code.push(OpCode::Ret);
    vm.is_cm = false;
    EvalResult::None
}

fn print_value_stack(vm: &mut VM) -> EvalResult {
    let stack = &vm.stack;
    print!("[ ");
    for v in stack {
        print!("{} ", v);
    }
    print!("]\n");
    io::stdout().flush().expect("stdout failed");
    EvalResult::None
}

fn print_return_stack(vm: &mut VM) -> EvalResult {
    for (i, r) in vm.ret.iter().enumerate() {
        println!("{} - {}", i, vm.i_2_n[*r]);
    }
    EvalResult::None
}

fn quit(vm: &mut VM) -> EvalResult {
    vm.dead = true;
    EvalResult::None
}

fn cm_set(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(p) => {
            vm.is_cm = if p == 0 { false } else { true };
            EvalResult::None
            },
        None => EvalResult::StackUnderflow
    }
}

fn cm_set_true(vm: &mut VM) -> EvalResult {
    vm.is_cm = true;
    EvalResult::None
}

fn cm_set_false(vm: &mut VM) -> EvalResult {
    vm.is_cm = false;
    EvalResult::None
}

fn cm_get(vm: &mut VM) -> EvalResult {
    vm.stack.push(if vm.is_cm { 1 } else { 0 }) ;
    EvalResult::None
}

fn get_addr(vm: &mut VM) -> EvalResult {
    let tok = vm.read_token();
    match tok {
        Some (tok) =>
            match vm.n_2_i.contains_key(&tok) {
                true => { vm.stack.push(vm.n_2_i[&tok]); EvalResult::None },
                false => EvalResult::WordNotFound(tok)
            },
        None => EvalResult::EmptyToken,
    }
}

fn opcode_literal(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(p) => {
            vm.code.push(OpCode::PushUSize(p));
            EvalResult::None
            },
        None => EvalResult::StackUnderflow
    }  
}

fn opcode_call(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(p) => {
            vm.code.push(OpCode::Call(p));
            EvalResult::None
            },
        None => EvalResult::StackUnderflow
    }  
}

fn opcode_cond(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(p) => {
            vm.code.push(OpCode::Cond(p));
            EvalResult::None
            },
        None => EvalResult::StackUnderflow
    }  
}

fn opcode_jmp(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(p) => {
            vm.code.push(OpCode::Jmp(p));
            EvalResult::None
            },
        None => EvalResult::StackUnderflow
    }  
}

fn opcode_ret(vm: &mut VM) -> EvalResult {
    vm.code.push(OpCode::Ret);
    EvalResult::None
}

fn add(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(a) =>
            match vm.stack.pop() {
                Some(b) => { vm.stack.push(a + b); EvalResult::None }
                None => EvalResult::StackUnderflow
            },
        None => EvalResult::StackUnderflow
    }
}

fn sub(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(a) =>
            match vm.stack.pop() {
                Some(b) => { vm.stack.push(a - b); EvalResult::None }
                None => EvalResult::StackUnderflow
            },
        None => EvalResult::StackUnderflow
    }
}

fn mul(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(a) =>
            match vm.stack.pop() {
                Some(b) => { vm.stack.push(a * b); EvalResult::None }
                None => EvalResult::StackUnderflow
            },
        None => EvalResult::StackUnderflow
    }
}

fn div(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(a) =>
            match vm.stack.pop() {
                Some(b) => { vm.stack.push(a / b); EvalResult::None }
                None => EvalResult::StackUnderflow
            },
        None => EvalResult::StackUnderflow
    }
}

fn modulo(vm: &mut VM) -> EvalResult {
    match vm.stack.pop() {
        Some(a) =>
            match vm.stack.pop() {
                Some(b) => { vm.stack.push(a % b); EvalResult::None }
                None => EvalResult::StackUnderflow
            },
        None => EvalResult::StackUnderflow
    }
}


static CORE_ENTRIES : [NativeEntry; 23] = [
    NativeEntry { name : "quit",    is_macro:  true, f: quit                 },  
    NativeEntry { name : ":",       is_macro:  true, f: define_word          }, 
    NativeEntry { name : "!",       is_macro:  true, f: define_immediate     },
    NativeEntry { name : ";",       is_macro:  true, f: eofn                 }, 
    NativeEntry { name : "cm.set",  is_macro:  true, f: cm_set               }, 
    NativeEntry { name : "cm.get",  is_macro:  true, f: cm_get               }, 
    NativeEntry { name : "cm.true", is_macro:  true, f: cm_set_true          }, 
    NativeEntry { name : "cm.false",is_macro:  true, f: cm_set_false         }, 
    NativeEntry { name : "@",       is_macro:  true, f: get_addr             },
    NativeEntry { name : ".s",      is_macro: false, f: print_value_stack    }, 
    NativeEntry { name : ".r",      is_macro: false, f: print_return_stack   },     
    NativeEntry { name : "peekch",  is_macro: false, f: VM::peek_char        }, 
    NativeEntry { name : "getch",   is_macro: false, f: VM::read_char        }, 
    NativeEntry { name : "op.lit",  is_macro: false, f: opcode_literal       }, 
    NativeEntry { name : "op.call", is_macro: false, f: opcode_call          }, 
    NativeEntry { name : "op.cond", is_macro: false, f: opcode_cond          }, 
    NativeEntry { name : "op.jmp",  is_macro: false, f: opcode_jmp           }, 
    NativeEntry { name : "op.ret",  is_macro: false, f: opcode_ret           },
    NativeEntry { name : "+",       is_macro: false, f: add                  },    
    NativeEntry { name : "-",       is_macro: false, f: sub                  },    
    NativeEntry { name : "*",       is_macro: false, f: mul                  },    
    NativeEntry { name : "/",       is_macro: false, f: div                  },    
    NativeEntry { name : "%",       is_macro: false, f: modulo               },           
];

pub fn register(vm: &mut VM) {
    for ce in CORE_ENTRIES.iter() {
        vm.register_native(ce.name.to_string(), ce.is_macro, ce.f);
    }
}