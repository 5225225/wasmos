extern crate wabt;
extern crate wasmi;

use std::collections::HashMap;

use wasmi::{ImportResolver, ModuleInstance, NopExternals, RuntimeValue};

struct Imports {}

impl ImportResolver for Imports {
    fn resolve_func(
        &self,
        module_name: &str,
        field_name: &str,
        signature: &wasmi::Signature,
    ) -> std::result::Result<wasmi::FuncRef, wasmi::Error> {
        dbg!(module_name, field_name, signature);

        match module_name {
            "env" => match field_name {
                "_pragma" => {
                    use wasmi::ValueType::*;

                    return Ok(wasmi::FuncInstance::alloc_host(
                        wasmi::Signature::new(&[I32, I32][..], None),
                        1,
                    ));
                }
                "_create" => {
                    use wasmi::ValueType::*;

                    return Ok(wasmi::FuncInstance::alloc_host(
                        wasmi::Signature::new(&[I32, I32, I32][..], Some(I32)),
                        2,
                    ));
                }
                "_bind" => {
                    use wasmi::ValueType::*;

                    return Ok(wasmi::FuncInstance::alloc_host(
                        wasmi::Signature::new(&[I32, I32, I32, I32][..], Some(I32)),
                        3,
                    ));
                }
                "_spawn" => {
                    use wasmi::ValueType::*;

                    return Ok(wasmi::FuncInstance::alloc_host(
                        wasmi::Signature::new(&[I32][..], Some(I32)),
                        4,
                    ));
                }
                "_invoke" => {
                    use wasmi::ValueType::*;

                    return Ok(wasmi::FuncInstance::alloc_host(
                        wasmi::Signature::new(&[I32, I32, I32, I32, I32][..], Some(I32)),
                        5,
                    ));
                }
                _ => {}
            },
            _ => {}
        }

        Err(wasmi::Error::Instantiation(format!(
            "could not find {} in module {}",
            field_name, module_name
        )))
    }

    fn resolve_global(
        &self,
        _module_name: &str,
        _field_name: &str,
        _descriptor: &wasmi::GlobalDescriptor,
    ) -> std::result::Result<wasmi::GlobalRef, wasmi::Error> {
        todo!()
    }

    fn resolve_memory(
        &self,
        _module_name: &str,
        _field_name: &str,
        _descriptor: &wasmi::MemoryDescriptor,
    ) -> std::result::Result<wasmi::MemoryRef, wasmi::Error> {
        todo!()
    }

    fn resolve_table(
        &self,
        _module_name: &str,
        _field_name: &str,
        _descriptor: &wasmi::TableDescriptor,
    ) -> std::result::Result<wasmi::TableRef, wasmi::Error> {
        todo!()
    }
}

struct HostExternals {
    processes: HashMap<u32, Process>,
    spawned_processes: HashMap<u32, SpawnedProcess>,
    new_idx: u32,
    module: wasmi::ModuleRef,
    mem: wasmi::MemoryRef,
}

impl HostExternals {
    fn new(module: wasmi::ModuleRef) -> Self {
        HostExternals {
            new_idx: 0,
            processes: Default::default(),
            spawned_processes: Default::default(),
            mem: module
                .export_by_name("memory")
                .unwrap()
                .as_memory()
                .unwrap()
                .clone(),
            module,
        }
    }
}

struct Process {
    module: wasmi::Module,
    bindings: BindingSet,
}

struct SpawnedProcess {
    module: wasmi::ModuleRef,
}

#[derive(Default)]
struct BindingSet {
    bindings: HashMap<String, wasmi::FuncRef>,
}

impl wasmi::ModuleImportResolver for BindingSet {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &wasmi::Signature,
    ) -> Result<wasmi::FuncRef, wasmi::Error> {
        Ok(self.bindings[field_name].clone())
    }
}

impl wasmi::Externals for HostExternals {
    fn invoke_index(
        &mut self,
        index: usize,
        args: wasmi::RuntimeArgs,
    ) -> Result<Option<wasmi::RuntimeValue>, wasmi::Trap> {
        dbg!(&index, &args);

        match index {
            1 => Ok(None),
            2 => {
                self.new_idx += 16;
                let idx = self.new_idx | 0b0001;

                let bytecode = self
                    .mem
                    .get(args.nth(0), args.nth::<u32>(1) as usize)
                    .unwrap();

                let module = match wasmi::Module::from_buffer(&bytecode) {
                    Ok(m) => m,
                    Err(_) => {
                        return Ok(Some(0.into()));
                    }
                };

                let proc = Process {
                    module,
                    bindings: Default::default(),
                };

                self.processes.insert(idx, proc);

                Ok(Some(idx.into()))
            }
            3 => {
                let handle: u32 = args.nth(0);
                let fn_name_ptr: u32 = args.nth(1);
                let fn_name_length: u32 = args.nth(2);
                let fnptr: u32 = args.nth(3);

                let proc = self.processes.get_mut(&handle).unwrap();
                let fn_name = self.mem.get(fn_name_ptr, fn_name_length as usize).unwrap();

                let fn_name_str = String::from_utf8(fn_name).unwrap();

                let exp = self
                    .module
                    .export_by_name("__indirect_function_table")
                    .unwrap();

                let table = exp.as_table().unwrap();

                assert!(
                    fnptr < 1024,
                    "you probably passed a *pointer* to memory not a table index. don't do that."
                );

                let fnref = table.get(fnptr).unwrap().unwrap();

                proc.bindings.bindings.insert(fn_name_str, fnref);

                Ok(Some(0.into()))
            }
            4 => {
                let handle: u32 = args.nth(0);

                dbg!(handle);

                self.new_idx += 16;
                let idx = self.new_idx | 0b0010;

                let proc = self.processes.remove(&handle).unwrap();

                let imports = wasmi::ImportsBuilder::default().with_resolver("env", &proc.bindings);

                let mi = ModuleInstance::new(&proc.module, &imports)
                    .unwrap()
                    .assert_no_start();

                let sp = SpawnedProcess { module: mi };

                self.spawned_processes.insert(idx, sp);

                Ok(Some(idx.into()))
            }
            5 => {
                let handle: u32 = args.nth(0);
                let fn_name_ptr: u32 = args.nth(1);
                let fn_name_length: u32 = args.nth(2);
                let arg_ptr: u32 = args.nth(3);
                let result_ptr: u32 = args.nth(4);

                let fn_name_bytes = self.mem.get(fn_name_ptr, fn_name_length as usize).unwrap();
                let fn_name_str = String::from_utf8(fn_name_bytes).unwrap();

                let sp = self.spawned_processes.get_mut(&handle).unwrap();

                let exp = sp.module.export_by_name(&fn_name_str).unwrap();
                let func = exp.as_func().unwrap();

                let mut idx = arg_ptr;

                let mut runtime_values = Vec::<wasmi::RuntimeValue>::new();
                for param in func.signature().params() {
                    use wasmi::nan_preserving_float::{F32, F64};
                    use wasmi::ValueType;

                    let rtv = match param {
                        ValueType::I32 => self.mem.get_value::<i32>(idx).unwrap().into(),
                        ValueType::I64 => self.mem.get_value::<i64>(idx).unwrap().into(),
                        ValueType::F32 => self.mem.get_value::<F32>(idx).unwrap().into(),
                        ValueType::F64 => self.mem.get_value::<F64>(idx).unwrap().into(),
                    };

                    // yes this means we have padding bytes for 32 bit types
                    // i do not care.
                    idx += 8;

                    runtime_values.push(rtv);
                }

                dbg!(&runtime_values);

                let result = sp
                    .module
                    .invoke_export(&fn_name_str, &runtime_values, &mut NopExternals)
                    .unwrap();

                match result {
                    Some(r) => {
                        use wasmi::RuntimeValue::*;
                        match r {
                            I32(v) => self.mem.set_value(result_ptr, v),
                            I64(v) => self.mem.set_value(result_ptr, v),
                            F32(v) => self.mem.set_value(result_ptr, v),
                            F64(v) => self.mem.set_value(result_ptr, v),
                        }
                        .unwrap();
                    }
                    None => {}
                };

                Ok(Some(0.into()))
            }
            _ => panic!("Unimplemented function at {}", index),
        }
    }
}

fn main() {
    // if you're getting a build error here, go to 'wasm' and do `cargo build --release`
    let wasm_binary = include_bytes!(
        "../wasm/target/wasm32-unknown-unknown/release/hello_world.wasm"
    );

    // Load wasm binary and prepare it for instantiation.
    let module = wasmi::Module::from_buffer(wasm_binary.as_ref()).expect("failed to load wasm");

    let instance = ModuleInstance::new(&module, &Imports {})
        .expect("failed to instantiate wasm module")
        .assert_no_start();

    let mut externals = HostExternals::new(instance.clone());

    assert_eq!(
        instance
            .invoke_export("test", &[], &mut externals,)
            .expect("failed to execute export"),
        Some(RuntimeValue::I32(1337)),
    );
}
