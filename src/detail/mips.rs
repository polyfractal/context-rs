#[repr(C)]
#[derive(Debug)]
pub struct Registers([libc::uintptr_t; 32]);

impl Registers {
    pub fn new() -> Registers {
        Registers([0; 32])
    }
}

pub fn initialize_call_frame(regs: &mut Registers, fptr: InitFn, arg: usize, thunkptr: *mut libc::c_void, sp: *mut usize) {
    let sp = align_down(sp);
    // sp of mips o32 is 8-byte aligned
    let sp = mut_offset(sp, -2);

    // The final return address. 0 indicates the bottom of the stack
    unsafe { *sp = 0; }

    let &mut Registers(ref mut regs) = regs;

    regs[4] = arg as libc::uintptr_t;
    regs[5] = thunkptr as libc::uintptr_t;
    regs[29] = sp as libc::uintptr_t;
    regs[25] = fptr as libc::uintptr_t;
    regs[31] = fptr as libc::uintptr_t;
}
