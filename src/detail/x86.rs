use libc;

use detail::{align_down, mut_offset};
use context::InitFn;

#[repr(C)]
#[derive(Debug)]
struct Registers {
    eax: u32, ebx: u32, ecx: u32, edx: u32,
    ebp: u32, esi: u32, edi: u32, esp: u32,
    cs: u16, ds: u16, ss: u16, es: u16, fs: u16, gs: u16,
    eflags: u32, eip: u32
}

pub impl Registers {
    fn new() -> Registers {
        Registers {
            eax: 0, ebx: 0, ecx: 0, edx: 0,
            ebp: 0, esi: 0, edi: 0, esp: 0,
            cs: 0, ds: 0, ss: 0, es: 0, fs: 0, gs: 0,
            eflags: 0, eip: 0,
        }
    }
}

pub fn initialize_call_frame(regs: &mut Registers, fptr: InitFn, arg1: usize, arg2: *mut libc::c_void, sp: *mut usize) {
    // x86 has interesting stack alignment requirements, so do some alignment
    // plus some offsetting to figure out what the actual stack should be.
    let sp = align_down(sp);
    let sp = mut_offset(sp, -4); // dunno why offset 4, TODO
/*
    |----------------+----------------------+---------------+-------|
    | position(high) | data                 | comment       |       |
    |----------------+----------------------+---------------+-------|
    |             +3 | null                 |               |       |
    |             +2 | boxed_thunk_ptr      |               |       |
    |             +1 | argptr               | taskhandleptr |       |
    |              0 | retaddr(0) no return |               | <- sp |
    |----------------+----------------------+---------------+-------|
*/
    unsafe { *mut_offset(sp, 2) = arg2 as usize };
    unsafe { *mut_offset(sp, 1) = arg1 as usize };
    unsafe { *mut_offset(sp, 0) = 0 }; // The final return address, 0 because of !

    regs.esp = sp as u32;
    regs.eip = fptr as u32;

    // Last base pointer on the stack is 0
    regs.ebp = 0;
}

#[cold]
#[inline(never)]
pub unsafe fn swap_registers(_out_regs: *mut Registers, _in_regs: *const Registers) {

}

#[cold]
#[inline(never)]
pub unsafe fn load_registers(_in_regs: *const Registers) {

}

#[cold]
#[inline(never)]
pub unsafe fn save_registers(_out_regs: *mut Registers) {

}
