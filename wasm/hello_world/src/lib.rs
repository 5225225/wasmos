extern crate wee_alloc;

use std::mem::MaybeUninit;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern "C" {
    // Hint. Used for debugging. Will never cause side effects, must act as if it's defined as a
    // no-op.
    //
    // But can be used for debug logs... and any other ignorable hints.
    fn _pragma(val: u32, value: *const u8);

    // Creates a process using the wasm bytecode. Would write a result into result if I implemented
    // that yet...
    fn _create(bytecode: *const u8, bytecode_length: u32, result: *mut u32) -> u32; // handle to create process

    // Binds a function by the name fn_name to the function func
    // func must be in the table so that we can pass it to the new process
    fn _bind(handle: u32, fn_name: *const u8, fn_name_length: u32, func: *const u8) -> u32;

    // Actually creates a moduleinstance from the process. Returns a *new* handle type of *spawned
    // process*.
    fn _spawn(handle: u32) -> u32;

    // Invokes a specific function on a spawned process.
    fn _invoke(handle: u32, fn_name: *const u8, fn_name_length: u32, arguments_ptr: *const u64, result: *mut u8) -> u32;

    // todo: introspection APIs so you can know what some bytecode wants/exports
}

#[no_mangle]
pub extern "C" fn test() -> i32 {

    // exports 1 function test() -> i32 { 1337 }
    // no dependencies
    let bytecode = vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x08, 0x01, 0x04, 0x74,
        0x65, 0x73, 0x74, 0x00, 0x00, 0x0a, 0x07, 0x01,
        0x05, 0x00, 0x41, 0xb9, 0x0a, 0x0b];


    // asks for one import, env.frob(i32) -> i32
    // has one export, add(i32) -> i32
    // returns lhs + frob(rhs)
    // 
    let more_advanced_bytecode = vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x0c, 0x02, 0x60, 0x01, 0x7f, 0x01, 0x7f,
        0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, 0x02, 0x0c,
        0x01, 0x03, 0x65, 0x6e, 0x76, 0x04, 0x66, 0x72,
        0x6f, 0x62, 0x00, 0x00, 0x03, 0x02, 0x01, 0x01,
        0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00,
        0x01, 0x0a, 0x0b, 0x01, 0x09, 0x00, 0x20, 0x00,
        0x20, 0x01, 0x10, 0x00, 0x6a, 0x0b,
    ];



    /*
    unsafe {
        let handle = _create(bytecode.as_ptr(), bytecode.len() as u32, std::ptr::null_mut());

        let fn_name = "test";
        let mut output: MaybeUninit<i32> = MaybeUninit::uninit();

        let spawned_handle = _spawn(handle);

        _invoke(spawned_handle, 
            fn_name.as_ptr(), fn_name.len() as u32,
            output.as_mut_ptr().cast::<u8>());

        return output.assume_init();
    }*/

    unsafe {
        let handle = _create(more_advanced_bytecode.as_ptr(), more_advanced_bytecode.len() as u32, std::ptr::null_mut());

        let binding_name = "frob";
        let binding_name_ptr = binding_name.as_ptr();
        let binding_name_len = binding_name.len();

        // We don't have the support for the host to call using a captured environment
        // So we have to use a function pointer here (Which directly corresponds to a index)
        let func: fn(i32) -> i32 = |x| x * 10 + 5;

        _bind(handle, binding_name_ptr, binding_name_len as u32, func as *const u8);

        let new_handle = _spawn(handle);

        let invoking_name = "add";
        let invoking_name_ptr = invoking_name.as_ptr();
        let invoking_name_len = invoking_name.len();

        // arguments are widened to 64 bits
        // how do you know if you're not passing in enough arguments? 
        // that is correct. you do not.
        //
        // (until i add either a argument_len that trips an assert, or reflection and just call it
        // UB to not check)
        let args: [u64; 2] = [132 as u64, 120 as u64];


        let mut output: MaybeUninit<i32> = MaybeUninit::uninit();
        _invoke(new_handle, invoking_name_ptr, invoking_name_len as u32, args.as_ptr(), output.as_mut_ptr().cast::<u8>());

        assert!(output.assume_init() == 1337);

        return output.assume_init();
    }
}
