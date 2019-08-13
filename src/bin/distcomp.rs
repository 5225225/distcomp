use distcomp::{ApplicationId, Journal, SqliteJournal};
use uuid::Uuid;
use wasmi::{ImportsBuilder, ModuleInstance};

struct HostExternals {
    appid: ApplicationId,
    journal: Box<dyn Journal>,
    memory: wasmi::MemoryRef,
}

impl wasmi::Externals for HostExternals {
    fn invoke_index(
        &mut self,
        index: usize,
        args: wasmi::RuntimeArgs,
    ) -> Result<Option<wasmi::RuntimeValue>, wasmi::Trap> {
        use wasmi::TrapKind::*;

        match index {
            1 => {
                let addr = args.nth_checked::<u32>(0)?;

                let key = self
                    .memory
                    .get_value(addr)
                    .map_err(|_| MemoryAccessOutOfBounds)?;

                self.journal.commit_self(self.appid, key);

                Ok(None)
            }
            2 => {
                let addr = args.nth_checked::<u32>(0)?;

                let head = self.journal.get_state(self.appid);

                if let Some(head) = head {
                    self.memory
                        .set_value(addr, head)
                        .map_err(|_| MemoryAccessOutOfBounds)?;

                    Ok(Some(0.into()))
                } else {
                    Ok(Some(1.into()))
                }
            }
            3 => {
                let key_addr = args.nth_checked::<u32>(0)?;
                let offset = args.nth_checked::<u32>(1)? as usize;
                let len = args.nth_checked::<u32>(2)? as usize;
                let dest = args.nth_checked::<u32>(3)?;

                let key = self
                    .memory
                    .get_value(key_addr)
                    .map_err(|_| MemoryAccessOutOfBounds)?;

                let data = self.journal.cas_get(key);

                if let Some(data) = data {
                    let start: usize = data.len().min(offset);
                    let stop: usize = data.len().min(offset + len);

                    let size: usize = (stop - start).min(len);

                    self.memory
                        .set(dest, &data[start..start + size])
                        .map_err(|_| MemoryAccessOutOfBounds)?;

                    Ok(Some((size as i64).into()))
                } else {
                    return Ok(Some((-1 as i64).into()));
                }
            }
            4 => {
                let src = args.nth_checked::<u32>(0)?;

                let len = args.nth_checked::<u32>(1)?;

                let key_addr = args.nth_checked::<u32>(2)?;

                let data = self
                    .memory
                    .get(src, len as usize)
                    .map_err(|_| MemoryAccessOutOfBounds)?;

                let key = self.journal.cas_put(data);

                self.memory
                    .set_value(key_addr, key)
                    .map_err(|_| MemoryAccessOutOfBounds)?;

                Ok(None)
            }
            5 => {
                let src = args.nth_checked::<u32>(0)?;
                let len = args.nth_checked::<u32>(1)?;

                let data = self
                    .memory
                    .get(src, len as usize)
                    .map_err(|_| MemoryAccessOutOfBounds)?;

                let output_str = String::from_utf8(data).unwrap();

                print!("{}", output_str);

                Ok(Some(len.into()))
            }
            _ => panic!("Unimplemented function at {}", index),
        }
    }
}

struct Resolver {}

impl wasmi::ModuleImportResolver for Resolver {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &wasmi::Signature,
    ) -> Result<wasmi::FuncRef, wasmi::Error> {
        use wasmi::ValueType::*;

        match field_name {
            "update_state" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32][..], None),
                    1,
                ));
            }
            "get_state" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32][..], Some(I32)),
                    2,
                ));
            }
            "cas_get" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32, I32, I32, I32][..], Some(I64)),
                    3,
                ));
            }
            "cas_put" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32, I32, I32][..], None),
                    4,
                ));
            }
            "output" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32, I32][..], Some(I32)),
                    5,
                ));
            }
            _ => {
                return Err(wasmi::Error::Instantiation("Failed to resolve".to_string()));
            }
        }
    }
}

fn func_main(appid: ApplicationId, journal: Box<Journal>) {
    let wasm_binary =
        include_bytes!("../../applications/target/wasm32-unknown-unknown/debug/notepad.wasm");

    // Load wasm binary and prepare it for instantiation.
    let module = wasmi::Module::from_buffer(&wasm_binary[..]).expect("failed to load wasm");

    let resolver = Resolver {};

    let imports = ImportsBuilder::new().with_resolver("env", &resolver);

    // Instantiate a module with empty imports and
    // assert that there is no `start` function.
    let instance = ModuleInstance::new(&module, &imports)
        .expect("failed to instantiate wasm module")
        .assert_no_start();

    let memory = instance
        .export_by_name("memory")
        .expect("`memory` export not found")
        .as_memory()
        .expect("export name `memory` is not of memory type")
        .clone();

    let mut externals = HostExternals {
        appid,
        journal,
        memory,
    };

    // Finally, invoke the exported function "test" with no parameters
    // and empty external function executor.
    assert_eq!(
        instance
            .invoke_export("main", &[], &mut externals,)
            .expect("failed to execute export"),
        None,
    );
}

fn main() {
    better_panic::install();

    let journal = SqliteJournal::new("sqlite.db");

    let appid = ApplicationId(Uuid::parse_str("f524b42d-7108-4489-8c84-988462634d39").unwrap());

    func_main(appid, Box::new(journal));
}
