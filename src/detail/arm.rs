
#[repr(C)]
#[derive(Debug)]
pub struct Registers([libc::uintptr_t; 32]);

impl Registers {
    pub fn new() -> Registers {
        Registers([0; 32])
    }
}

pub fn initialize_call_frame(regs: &mut Registers, fptr: InitFn, arg: usize, arg2: *mut libc::c_void, sp: *mut usize) {
    extern { fn rust_bootstrap_green_task(); } // same as the x64 arch

    let sp = align_down(sp);
    // sp of arm eabi is 8-byte aligned
    let sp = mut_offset(sp, -2);

    // The final return address. 0 indicates the bottom of the stack
    unsafe { *sp = 0; }

    let &mut Registers(ref mut regs) = regs;

    // ARM uses the same technique as x86_64 to have a landing pad for the start
    // of all new green tasks. Neither r1/r2 are saved on a context switch, so
    // the shim will copy r3/r4 into r1/r2 and then execute the function in r5
    regs[0] = arg as libc::uintptr_t;              // r0
    regs[3] = arg2 as libc::uintptr_t;         // r3
    regs[5] = fptr as libc::uintptr_t;             // r5
    regs[13] = sp as libc::uintptr_t;                          // #52 sp, r13
    regs[14] = rust_bootstrap_green_task as libc::uintptr_t;   // #56 pc, r14 --> lr
}
