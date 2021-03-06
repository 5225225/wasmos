#![no_std]
extern crate alloc;

extern crate wee_alloc;

use core::mem::MaybeUninit;

use wasmcorelib::{_bind, _create, _invoke, _spawn, params};

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[no_mangle]
pub extern "C" fn test() -> i32 {
    // exports 1 function test() -> i32 { 1337 }
    // no dependencies
    let bytecode = alloc::vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f,
        0x03, 0x02, 0x01, 0x00, 0x07, 0x08, 0x01, 0x04, 0x74, 0x65, 0x73, 0x74, 0x00, 0x00, 0x0a,
        0x07, 0x01, 0x05, 0x00, 0x41, 0xb9, 0x0a, 0x0b
    ];

    // asks for one import, env.frob(i32) -> i32
    // has one export, add(i32) -> i32
    // returns lhs + frob(rhs)
    //
    let more_advanced_bytecode = alloc::vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0c, 0x02, 0x60, 0x01, 0x7f, 0x01,
        0x7f, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, 0x02, 0x0c, 0x01, 0x03, 0x65, 0x6e, 0x76, 0x04,
        0x66, 0x72, 0x6f, 0x62, 0x00, 0x00, 0x03, 0x02, 0x01, 0x01, 0x07, 0x07, 0x01, 0x03, 0x61,
        0x64, 0x64, 0x00, 0x01, 0x0a, 0x0b, 0x01, 0x09, 0x00, 0x20, 0x00, 0x20, 0x01, 0x10, 0x00,
        0x6a, 0x0b,
    ];

    unsafe {
        let handle = _create(
            bytecode.as_ptr(),
            bytecode.len() as u32,
            core::ptr::null_mut(),
        );

        let fn_name = "test";
        let mut output: MaybeUninit<i32> = MaybeUninit::uninit();

        let spawned_handle = _spawn(handle);

        _invoke(
            spawned_handle,
            fn_name.as_ptr(),
            fn_name.len() as u32,
            core::ptr::null(),
            core::ptr::null(),
            0,
            output.as_mut_ptr().cast::<u64>(),
        );

        assert!(output.assume_init() == 1337);
    }

    unsafe {
        let handle = _create(
            more_advanced_bytecode.as_ptr(),
            more_advanced_bytecode.len() as u32,
            core::ptr::null_mut(),
        );

        let binding_name = "frob";
        let binding_name_ptr = binding_name.as_ptr();
        let binding_name_len = binding_name.len();

        // We don't have the support for the host to call using a captured environment
        // So we have to use a function pointer here (Which directly corresponds to a index)
        let func: fn(i32) -> i32 = |x| x * 10 + 5;

        _bind(
            handle,
            binding_name_ptr,
            binding_name_len as u32,
            func as *const u8,
        );

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
        let argtypes = b"ii";

        let mut output: MaybeUninit<i32> = MaybeUninit::uninit();
        _invoke(
            new_handle,
            invoking_name_ptr,
            invoking_name_len as u32,
            args.as_ptr(),
            argtypes.as_ptr(),
            args.len() as u32,
            output.as_mut_ptr().cast::<u64>(),
        );

        assert!(output.assume_init() == 1337);
    }

    let mut handle = wasmcorelib::create(&more_advanced_bytecode).unwrap();

    handle
        .bind("frob", (|x| x * 10 + 5) as fn(i32) -> i32)
        .unwrap();
    let mut proc = handle.spawn().unwrap();

    assert!(proc.invoke("add", params!(132_u32, 120_u32)).unwrap() as i32 == 1337);

    1337
}
