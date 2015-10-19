// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use stack::Stack;
use std::usize;

use detail::{Registers, initialize_call_frame, swap_registers, load_registers, save_registers};

use libc;

use sys;

#[derive(Debug)]
pub struct Context {
    /// Hold the registers while the task or scheduler is suspended
    regs: Registers,
    /// Lower bound and upper bound for the stack
    stack_bounds: Option<(usize, usize)>,
}

pub type InitFn = extern "C" fn(usize, *mut libc::c_void) -> !; // first argument is task handle, second is thunk ptr

impl Context {
    pub fn empty() -> Context {
        Context {
            regs: Registers::new(),
            stack_bounds: None,
        }
    }

    /// Create a new context that will resume execution by running start
    ///
    /// The `init` function will be run with `arg` and the `start` procedure
    /// split up into code and env pointers. It is required that the `init`
    /// function never return.
    ///
    /// FIXME: this is basically an awful the interface. The main reason for
    ///        this is to reduce the number of allocations made when a green
    ///        task is spawned as much as possible
    pub fn new(init: InitFn, arg: usize, start: *mut libc::c_void, stack: &mut Stack) -> Context {
        let mut ctx = Context::empty();
        ctx.init_with(init, arg, start, stack);
        ctx
    }

    pub fn init_with(&mut self, init: InitFn, arg: usize, start: *mut libc::c_void, stack: &mut Stack) {
        let sp: *const usize = stack.end();
        let sp: *mut usize = sp as *mut usize;
        // Save and then immediately load the current context,
        // which we will then modify to call the given function when restored

        initialize_call_frame(&mut self.regs, init, arg, start, sp);

        // Scheduler tasks don't have a stack in the "we allocated it" sense,
        // but rather they run on pthreads stacks. We have complete control over
        // them in terms of the code running on them (and hopefully they don't
        // overflow). Additionally, their coroutine stacks are listed as being
        // zero-length, so that's how we detect what's what here.
        let stack_base: *const usize = stack.start();
        self.stack_bounds =
            if sp as libc::uintptr_t == stack_base as libc::uintptr_t {
                None
            } else {
                Some((stack_base as usize, sp as usize))
            };
    }

    /// Switch contexts

    /// Suspend the current execution context and resume another by
    /// saving the registers values of the executing thread to a Context
    /// then loading the registers from a previously saved Context.
    pub fn swap(out_context: &mut Context, in_context: &Context) {
        debug!("swapping contexts");
        let out_regs: &mut Registers = match out_context {
            &mut Context { regs: ref mut r, .. } => r
        };
        let in_regs: &Registers = match in_context {
            &Context { regs: ref r, .. } => r
        };

        debug!("noting the stack limit and doing raw swap");

        unsafe {
            // Right before we switch to the new context, set the new context's
            // stack limit in the OS-specified TLS slot. This also  means that
            // we cannot call any more rust functions after record_stack_bounds
            // returns because they would all likely fail due to the limit being
            // invalid for the current task. Lucky for us `rust_swap_registers`
            // is a C function so we don't have to worry about that!
            //
            match in_context.stack_bounds {
                Some((lo, hi)) => sys::stack::record_rust_managed_stack_bounds(lo, hi),
                // If we're going back to one of the original contexts or
                // something that's possibly not a "normal task", then reset
                // the stack limit to 0 to make morestack never fail
                None => sys::stack::record_rust_managed_stack_bounds(0, usize::MAX),
            }
            swap_registers(out_regs, in_regs)
        }
    }

    /// Save the current context.
    #[inline(always)]
    pub fn save(context: &mut Context) {
        let regs: &mut Registers = &mut context.regs;

        unsafe {
            save_registers(regs);
        }
    }

    /// Load the context and switch. This function will never return.
    ///
    /// It is equivalent to `Context::swap(&mut dummy_context, &to_context)`.
    pub fn load(to_context: &Context) -> ! {
        let regs: &Registers = &to_context.regs;

        unsafe {
            // Right before we switch to the new context, set the new context's
            // stack limit in the OS-specified TLS slot. This also  means that
            // we cannot call any more rust functions after record_stack_bounds
            // returns because they would all likely fail due to the limit being
            // invalid for the current task. Lucky for us `rust_swap_registers`
            // is a C function so we don't have to worry about that!
            //
            match to_context.stack_bounds {
                Some((lo, hi)) => sys::stack::record_rust_managed_stack_bounds(lo, hi),
                // If we're going back to one of the original contexts or
                // something that's possibly not a "normal task", then reset
                // the stack limit to 0 to make morestack never fail
                None => sys::stack::record_rust_managed_stack_bounds(0, usize::MAX),
            }

            load_registers(regs);
        }

        unreachable!("Should never reach here");
    }
}

#[cfg(test)]
mod test {
    use libc;

    use std::mem::transmute;

    use stack::Stack;
    use context::Context;

    const MIN_STACK: usize = 2 * 1024 * 1024;

    extern "C" fn init_fn(arg: usize, f: *mut libc::c_void) -> ! {
        let func: fn() = unsafe {
            transmute(f)
        };
        func();

        let ctx: &Context = unsafe { transmute(arg) };
        Context::load(ctx);
    }

    #[test]
    fn test_swap_context() {
        let mut cur = Context::empty();

        fn callback() {}

        let mut stk = Stack::new(MIN_STACK);
        let ctx = Context::new(init_fn, unsafe { transmute(&cur) }, unsafe { transmute(callback) }, &mut stk);

        Context::swap(&mut cur, &ctx);
    }

    #[test]
    fn test_load_save_context() {
        let mut cur = Context::empty();

        fn callback() {}

        let mut stk = Stack::new(MIN_STACK);
        let ctx = Context::new(init_fn, unsafe { transmute(&cur) }, unsafe { transmute(callback) }, &mut stk);

        let mut _no_use = Box::new(true);

        Context::save(&mut cur);
        if *_no_use {
            *_no_use = false;
            Context::load(&ctx);
        }
    }
}
