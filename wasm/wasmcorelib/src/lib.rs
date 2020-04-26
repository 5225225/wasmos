#![feature(alloc_error_handler)]
#![no_std]

use core::convert::TryInto;
extern crate alloc;

extern "C" {
    // Hint. Used for debugging. Will never cause side effects, must act as if it's defined as a
    // no-op.
    //
    // But can be used for debug logs... and any other ignorable hints.
    pub fn _pragma(val: u32, value: *const u8);

    // Creates a process using the wasm bytecode. Would write a result into result if I implemented
    // that yet...
    pub fn _create(bytecode: *const u8, bytecode_length: u32, result: *mut u32) -> u32; // handle to create process

    // Binds a function by the name fn_name to the function func
    // func must be in the table so that we can pass it to the new process
    pub fn _bind(handle: u32, fn_name: *const u8, fn_name_length: u32, func: *const u8) -> u32;

    // Actually creates a moduleinstance from the process. Returns a *new* handle type of *spawned
    // process*.
    pub fn _spawn(handle: u32) -> u32;

    // Invokes a specific function on a spawned process.
    pub fn _invoke(
        handle: u32,
        fn_name: *const u8,
        fn_name_length: u32,
        arguments_ptr: *const u64,
        argtypes_ptr: *const u8,
        arglen: u32,
        result: *mut u64,
    ) -> u32;

// todo: introspection APIs so you can know what some bytecode wants/exports
}

pub struct CreateProcessHandle(u32);

#[derive(Debug)]
pub enum CreateProcessError {
    /// Tried to create a process with a bytecode length over 4GB (won't fit in a u32)
    TooLong,
    Unknown(u32),
}

pub fn create(bytecode: &[u8]) -> Result<CreateProcessHandle, CreateProcessError> {
    unsafe {
        let mut err_code: u32 = 0;
        let result = _create(
            bytecode.as_ptr(),
            bytecode
                .len()
                .try_into()
                .map_err(|_| CreateProcessError::TooLong)?,
            &mut err_code as *mut u32,
        );

        if err_code == 0 {
            Ok(CreateProcessHandle(result))
        } else {
            Err(CreateProcessError::Unknown(err_code))
        }
    }
}

pub unsafe trait IntoFnHandle {
    fn into_handle(self) -> u32;
}

pub unsafe trait Arg {}

unsafe impl Arg for f32 {}
unsafe impl Arg for f64 {}
unsafe impl Arg for i32 {}
unsafe impl Arg for i64 {}
unsafe impl Arg for u32 {}
unsafe impl Arg for u64 {}

unsafe impl IntoFnHandle for fn() {
    fn into_handle(self) -> u32 {
        unsafe { core::mem::transmute(self) }
    }
}
unsafe impl<R: Arg> IntoFnHandle for fn() -> R {
    fn into_handle(self) -> u32 {
        unsafe { core::mem::transmute(self) }
    }
}

unsafe impl<T1: Arg> IntoFnHandle for fn(T1) {
    fn into_handle(self) -> u32 {
        unsafe { core::mem::transmute(self) }
    }
}
unsafe impl<T1: Arg, R: Arg> IntoFnHandle for fn(T1) -> R {
    fn into_handle(self) -> u32 {
        unsafe { core::mem::transmute(self) }
    }
}

unsafe impl<T1: Arg, T2: Arg> IntoFnHandle for fn(T1, T2) {
    fn into_handle(self) -> u32 {
        unsafe { core::mem::transmute(self) }
    }
}
unsafe impl<T1: Arg, T2: Arg, R: Arg> IntoFnHandle for fn(T1, T2) -> R {
    fn into_handle(self) -> u32 {
        unsafe { core::mem::transmute(self) }
    }
}

#[derive(Debug)]
pub enum BindProcessError {
    NameTooLong,
    Unknown(u32),
}

impl CreateProcessHandle {
    pub fn bind(&mut self, name: &str, to: impl IntoFnHandle) -> Result<(), BindProcessError> {
        let result;
        unsafe {
            result = _bind(
                self.0,
                name.as_ptr(),
                name.len()
                    .try_into()
                    .map_err(|_| BindProcessError::NameTooLong)?,
                to.into_handle() as *const u8,
            );
        }

        if result == 0 {
            Ok(())
        } else {
            Err(BindProcessError::Unknown(result))
        }
    }
}

pub struct ProcessHandle(u32);
#[derive(Debug)]
pub enum SpawnError {}

impl CreateProcessHandle {
    pub fn spawn(self) -> Result<ProcessHandle, SpawnError> {
        let new_handle;

        unsafe {
            new_handle = _spawn(self.0);
        }

        Ok(ProcessHandle(new_handle))
    }
}

#[macro_export]
macro_rules! params {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_vec = alloc::vec::Vec::new();
            let mut type_vec = alloc::vec::Vec::new();
            $(
                {
                    use $crate::IntoParam;
                    temp_vec.push($x.into_param());
                    type_vec.push($x.paramtype());
                }
            )*
            unsafe {
                $crate::Params::new(temp_vec, type_vec)
            }
        }
    };
}

pub struct Params(alloc::vec::Vec<u64>, alloc::vec::Vec<u8>);

impl Params {
    pub unsafe fn new(values: alloc::vec::Vec<u64>, types: alloc::vec::Vec<u8>) -> Self {
        Self(values, types)
    }
}

pub trait IntoParam {
    fn into_param(self) -> u64;
    fn paramtype(self) -> u8;
}

impl IntoParam for u32 {
    fn into_param(self) -> u64 {
        self as u64
    }

    fn paramtype(self) -> u8 {
        b'i'
    }
}

#[derive(Debug)]
pub enum InvokeError {}

impl ProcessHandle {
    pub fn invoke(&mut self, fn_name: &str, params: Params) -> Result<u64, InvokeError> {
        let mut result: core::mem::MaybeUninit<u64> = core::mem::MaybeUninit::uninit();

        unsafe {
            _invoke(
                self.0,
                fn_name.as_ptr(),
                fn_name.len().try_into().expect("function name too long"),
                params.0.as_ptr(),
                params.1.as_ptr(),
                params.0.len() as u32,
                result.as_mut_ptr(),
            );

            Ok(result.assume_init())
        }
    }
}

#[panic_handler]
fn panic_handler(_panic: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::wasm32::unreachable();
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("failed to allocate {:?}", layout);
}
