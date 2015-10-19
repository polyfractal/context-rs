use libc;
use simd;

use detail::{align_down, mut_offset};
use context::InitFn;

// windows requires saving more registers (both general and XMM), so the windows
// register context must be larger.
#[cfg(windows)]
#[repr(C)]
#[derive(Debug)]
pub struct Registers {
    gpr: [libc::uintptr_t; 14],
    _xmm: [simd::u32x4; 10]
}

#[cfg(windows)]
impl Registers {
    pub fn new() -> Registers {
        Registers {
            gpr: [0; 14],
            _xmm: [simd::u32x4::new(0,0,0,0); 10]
        }
    }
}

#[cfg(not(windows))]
#[repr(C)]
#[derive(Debug)]
pub struct Registers {
    gpr: [libc::uintptr_t; 10],
    _xmm: [simd::u32x4; 6]
}

#[cfg(not(windows))]
impl Registers {
    pub fn new() -> Registers {
        Registers {
            gpr: [0; 10],
            _xmm: [simd::u32x4::new(0,0,0,0); 6]
        }
    }
}

pub fn initialize_call_frame(regs: &mut Registers, fptr: InitFn, arg: usize, arg2: *mut libc::c_void, sp: *mut usize) {
    // extern { fn rust_bootstrap_green_task(); } // use an indirection because the call contract differences between windows and linux
    // TODO: use rust's condition compile attribute instead

    #[inline(never)]
    #[cfg(not(windows))]
    unsafe fn bootstrap_green_task() {
        asm!("
            mov %r12, %rdi
            mov %r13, %rsi
            mov %r14, 8(%rsp)
        "
        :
        :
        : "{rdi}", "{rsi}", "memory"
        : "volatile");
    }

    #[inline(never)]
    #[cfg(windows)]
    unsafe fn bootstrap_green_task() {
        asm!("
            mov %r12, %rcx
            mov %r13, %rdx
            mov %r14, 8(%rsp)
        "
        :
        :
        : "{rcx}", "{rdx}", "memory"
        : "volatile");
    }

    // Redefinitions from rt/arch/x86_64/regs.h
    static RUSTRT_RSP: usize = 1;
    static RUSTRT_IP: usize = 8;
    static RUSTRT_RBP: usize = 2;
    static RUSTRT_R12: usize = 4;
    static RUSTRT_R13: usize = 5;
    static RUSTRT_R14: usize = 6;
    // static RUSTRT_R15: usize = 7;

    let sp = align_down(sp);
    let sp = mut_offset(sp, -1);

    // The final return address. 0 indicates the bottom of the stack
    unsafe { *sp = 0; }

    debug!("creating call framenn");
    debug!("fptr {:#x}", fptr as libc::uintptr_t);
    debug!("arg {:#x}", arg);
    debug!("sp {:?}", sp);

    // These registers are frobbed by rust_bootstrap_green_task into the right
    // location so we can invoke the "real init function", `fptr`.
    regs.gpr[RUSTRT_R12] = arg as libc::uintptr_t;
    regs.gpr[RUSTRT_R13] = arg2 as libc::uintptr_t;
    regs.gpr[RUSTRT_R14] = fptr as libc::uintptr_t;

    // These registers are picked up by the regular context switch paths. These
    // will put us in "mostly the right context" except for frobbing all the
    // arguments to the right place. We have the small trampoline code inside of
    // rust_bootstrap_green_task to do that.
    regs.gpr[RUSTRT_RSP] = mut_offset(sp, -2) as libc::uintptr_t;
    regs.gpr[RUSTRT_IP] = bootstrap_green_task as libc::uintptr_t;

    unsafe {
        *mut_offset(sp, -2) = 0; // Frame pointer
        *mut_offset(sp, -1) = bootstrap_green_task as usize;
    }

    // Last base pointer on the stack should be 0
    regs.gpr[RUSTRT_RBP] = 0;
}

#[inline(never)]
#[cfg(not(windows))]
pub unsafe fn swap_registers(out_regs: *mut Registers, in_regs: *const Registers) {
    // The first argument is in %rdi, and the second one is in %rsi

    // Save registers
    asm!("
        mov %rbx, (0*8)(%rdi)
        mov %rsp, (1*8)(%rdi)
        mov %rbp, (2*8)(%rdi)
        mov %r12, (4*8)(%rdi)
        mov %r13, (5*8)(%rdi)
        mov %r14, (6*8)(%rdi)
        mov %r15, (7*8)(%rdi)

        mov %rdi, (3*8)(%rdi)

        movapd %xmm0, (10*8)(%rdi)
        movapd %xmm1, (12*8)(%rdi)
        movapd %xmm2, (14*8)(%rdi)
        movapd %xmm3, (16*8)(%rdi)
        movapd %xmm4, (18*8)(%rdi)
        movapd %xmm5, (20*8)(%rdi)


        mov (0*8)(%rsi), %rbx
        mov (1*8)(%rsi), %rsp
        mov (2*8)(%rsi), %rbp
        mov (4*8)(%rsi), %r12
        mov (5*8)(%rsi), %r13
        mov (6*8)(%rsi), %r14
        mov (7*8)(%rsi), %r15

        mov (3*8)(%rsi), %rdi

        movapd (10*8)(%rsi), %xmm0
        movapd (12*8)(%rsi), %xmm1
        movapd (14*8)(%rsi), %xmm2
        movapd (16*8)(%rsi), %xmm3
        movapd (18*8)(%rsi), %xmm4
        movapd (20*8)(%rsi), %xmm5
    "
    :
    : "{rdi}"(out_regs), "{rsi}"(in_regs)
    : "memory", "{rbx}", "{rsp}", "{rbp}", "{r12}", "{r13}", "{r14}", "{r15}",
      "{rdi}", "{xmm0}", "{xmm1}", "{xmm2}", "{xmm3}", "{xmm4}", "{xmm5}"
    : "volatile");
}

#[inline(never)]
#[cfg(windows)]
pub unsafe fn swap_registers(out_regs: *mut Registers, in_regs: *const Registers) {
    // The first argument is in %rcx, and the second one is in %rdx

    // Save registers
    asm!("
        mov %rbx, (0*8)(%rcx)
        mov %rsp, (1*8)(%rcx)
        mov %rbp, (2*8)(%rcx)
        mov %r12, (4*8)(%rcx)
        mov %r13, (5*8)(%rcx)
        mov %r14, (6*8)(%rcx)
        mov %r15, (7*8)(%rcx)

        mov %rdi, (9*8)(%rcx)
        mov %rsi, (10*8)(%rcx)

        mov %rcx, (3*8)(%rcx)

        movapd %xmm6, (14*8)(%rcx)
        movapd %xmm7, (16*8)(%rcx)
        movapd %xmm8, (18*8)(%rcx)
        movapd %xmm9, (20*8)(%rcx)
        movapd %xmm10, (22*8)(%rcx)
        movapd %xmm11, (24*8)(%rcx)
        movapd %xmm12, (26*8)(%rcx)
        movapd %xmm13, (28*8)(%rcx)
        movapd %xmm14, (30*8)(%rcx)
        movapd %xmm15, (32*8)(%rcx)


        mov (0*8)(%rdx), %rbx
        mov (1*8)(%rdx), %rsp
        mov (2*8)(%rdx), %rbp
        mov (4*8)(%rdx), %r12
        mov (5*8)(%rdx), %r13
        mov (6*8)(%rdx), %r14
        mov (7*8)(%rdx), %r15

        mov (9*8)(%rdx), %rdi
        mov (10*8)(%rdx), %rsi

        mov (3*8)(%rdx), %rcx

        movapd (14*8)(%rdx), %xmm6
        movapd (16*8)(%rdx), %xmm7
        movapd (18*8)(%rdx), %xmm8
        movapd (20*8)(%rdx), %xmm9
        movapd (22*8)(%rdx), %xmm10
        movapd (24*8)(%rdx), %xmm11
        movapd (26*8)(%rdx), %xmm12
        movapd (28*8)(%rdx), %xmm13
        movapd (30*8)(%rdx), %xmm14
        movapd (32*8)(%rdx), %xmm15
    "
    :
    : "{rcx}"(out_regs), "{rdx}"(in_regs)
    : "memory", "{rbx}", "{rsp}", "{rbp}", "{r12}", "{r13}", "{r14}", "{r15}",
      "{rdi}", "{rsi}", "{rcx}", "{xmm6}", "{xmm7}", "{xmm8}", "{xmm9}", "{xmm10}",
      "{xmm11}", "{xmm12}", "{xmm13}", "{xmm14}", "{xmm15}"
    : "volatile");
}

#[inline(never)]
#[cfg(not(windows))]
pub unsafe fn load_registers(in_regs: *const Registers) {
    asm!("
        mov (0*8)(%rdi), %rbx
        mov (1*8)(%rdi), %rsp
        mov (2*8)(%rdi), %rbp
        mov (4*8)(%rdi), %r12
        mov (5*8)(%rdi), %r13
        mov (6*8)(%rdi), %r14
        mov (7*8)(%rdi), %r15

        movapd (10*8)(%rdi), %xmm0
        movapd (12*8)(%rdi), %xmm1
        movapd (14*8)(%rdi), %xmm2
        movapd (16*8)(%rdi), %xmm3
        movapd (18*8)(%rdi), %xmm4
        movapd (20*8)(%rdi), %xmm5
    "
    :
    : "{rdi}"(in_regs)
    : "{rbx}", "{rsp}", "{rbp}",
      "{r12}", "{r13}", "{r14}", "{r15}",
      "{xmm0}", "{xmm1}", "{xmm2}", "{xmm3}",
      "{xmm4}", "{xmm5}"
    : "volatile");

    // Just return
}

#[inline(never)]
#[cfg(windows)]
pub unsafe fn load_registers(in_regs: *const Registers) {
    // Restore registers
    asm!("
        mov (0*8)(%rcx), %rbx
        mov (1*8)(%rcx), %rsp
        mov (2*8)(%rcx), %rbp
        mov (4*8)(%rcx), %r12
        mov (5*8)(%rcx), %r13
        mov (6*8)(%rcx), %r14
        mov (7*8)(%rcx), %r15

        mov (9*8)(%rcx), %rdi
        mov (10*8)(%rcx), %rsi

        movapd (14*8)(%rcx), %xmm6
        movapd (16*8)(%rcx), %xmm7
        movapd (18*8)(%rcx), %xmm8
        movapd (20*8)(%rcx), %xmm9
        movapd (22*8)(%rcx), %xmm10
        movapd (24*8)(%rcx), %xmm11
        movapd (26*8)(%rcx), %xmm12
        movapd (28*8)(%rcx), %xmm13
        movapd (30*8)(%rcx), %xmm14
        movapd (32*8)(%rcx), %xmm15
    "
    :
    : "{rcx}"(in_regs)
    : "{rbx}", "{rsp}", "{rbp}", "{r12}", "{r13}", "{r14}", "{r15}",
      "{rdi}", "{rsi}", "{rcx}", "{xmm6}", "{xmm7}", "{xmm8}", "{xmm9}", "{xmm10}",
      "{xmm11}", "{xmm12}", "{xmm13}", "{xmm14}", "{xmm15}"
    : "volatile");

    // Just return
}

#[inline(never)]
#[cfg(not(windows))]
pub unsafe fn save_registers(out_regs: *mut Registers) {
    // Save registers
    asm!("
        mov %rbx, (0*8)(%rdi)
        mov %rsp, (1*8)(%rdi)
        mov %rbp, (2*8)(%rdi)
        mov %r12, (4*8)(%rdi)
        mov %r13, (5*8)(%rdi)
        mov %r14, (6*8)(%rdi)
        mov %r15, (7*8)(%rdi)

        movapd %xmm0, (10*8)(%rdi)
        movapd %xmm1, (12*8)(%rdi)
        movapd %xmm2, (14*8)(%rdi)
        movapd %xmm3, (16*8)(%rdi)
        movapd %xmm4, (18*8)(%rdi)
        movapd %xmm5, (20*8)(%rdi)
    "
    :
    : "{rdi}"(out_regs)
    : "memory"
    : "volatile");
}

#[inline(never)]
#[cfg(windows)]
pub unsafe fn save_registers(out_regs: *mut Registers) {
    // Save registers
    asm!("
        mov %rbx, (0*8)(%rcx)
        mov %rsp, (1*8)(%rcx)
        mov %rbp, (2*8)(%rcx)
        mov %r12, (4*8)(%rcx)
        mov %r13, (5*8)(%rcx)
        mov %r14, (6*8)(%rcx)
        mov %r15, (7*8)(%rcx)

        mov %rdi, (9*8)(%rcx)
        mov %rsi, (10*8)(%rcx)

        movapd %xmm6, (14*8)(%rcx)
        movapd %xmm7, (16*8)(%rcx)
        movapd %xmm8, (18*8)(%rcx)
        movapd %xmm9, (20*8)(%rcx)
        movapd %xmm10, (22*8)(%rcx)
        movapd %xmm11, (24*8)(%rcx)
        movapd %xmm12, (26*8)(%rcx)
        movapd %xmm13, (28*8)(%rcx)
        movapd %xmm14, (30*8)(%rcx)
        movapd %xmm15, (32*8)(%rcx)
    "
    :
    : "{rcx}"(out_regs)
    : "memory"
    : "volatile");
}
