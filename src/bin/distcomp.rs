use distcomp::{ApplicationId, Journal, SqliteJournal, CASKey};
use std::io::Write;
use std::convert::TryInto;
use uuid::Uuid;
use wasmi::{ImportsBuilder, ModuleInstance};
use handlemanager::HandleManager;
use std::collections::HashMap;

#[macro_use]
extern crate derive_more;

enum Handle {
    Key(CASKey),
    Data(Vec<u8>),
}

impl Handle {
    fn as_key(&self) -> Option<&CASKey> {
        if let Handle::Key(k) = self {
            return Some(k);
        }

        None
    }

    fn as_data(&self) -> Option<&[u8]> {
        if let Handle::Data(k) = self {
            return Some(k);
        }

        None
    }
}

#[derive(Default)]
struct Handles {
    manager: HandleManager,
    handles: HashMap<usize, Handle>
}

impl Handles {
    fn insert(&mut self, h: Handle) -> Option<usize> {
        let id = self.manager.next()?;
        self.handles.insert(id, h);
        Some(id)
    }

    fn get(&mut self, k: usize) -> Option<&mut Handle> {
        self.handles.get_mut(&k)
    }

    fn release(&mut self, k: usize) {
        self.manager.release(k);
        self.handles.remove(&k);
    }
}

struct HostExternals {
    appid: ApplicationId,
    journal: Box<dyn Journal>,
    memory: wasmi::MemoryRef,
    handles: Handles,
}

#[derive(Debug, Display)]
#[display(fmt = "Invalid handle number {} used", _0)]
struct InvalidHandleError(u32);

impl wasmi::HostError for InvalidHandleError {} 

impl wasmi::Externals for HostExternals {
    fn invoke_index(
        &mut self,
        index: usize,
        args: wasmi::RuntimeArgs,
    ) -> Result<Option<wasmi::RuntimeValue>, wasmi::Trap> {
        use wasmi::TrapKind::*;
        use wasmi::RuntimeValue::*;

        match index {
            1 => {
                let handle = args.nth_checked::<u32>(0)?;

                let key = self.handles
                .get(handle.try_into().expect("could not convert a u32 to a usize?"))
                .ok_or(InvalidHandleError(handle))?
                .as_key().ok_or(InvalidHandleError(handle))?;

                self.journal.commit_self(self.appid, *key);

                Ok(None)
            }
            2 => {
                let head = self.journal.get_state(self.appid);


                if let Some(head) = head {
                    let handle = self.handles.insert(Handle::Key(head)).expect("failed to insert handle").try_into().expect("could not convert a handle to a u32");

                    Ok(Some(I32(handle)))
                } else {
                    Ok(Some(I32(0)))
                }
            }
            3 => {

                let handle = args.nth_checked::<u32>(0)?;

                let key = self.handles
                .get(handle.try_into().expect("could not convert a u32 to a usize?"))
                .ok_or(InvalidHandleError(handle))?
                .as_key().ok_or(InvalidHandleError(handle))?;

                let data = self.journal.cas_get(*key).expect("failed to get data").data;

                let handle: u32 = self.handles.insert(Handle::Data(data)).expect("failed to insert handle").try_into().expect("could not convert a handle to a u32");

                Ok(Some(handle.into()))
            }
            4 => {
                let src = args.nth_checked::<u32>(0)?;
                let len = args.nth_checked::<u32>(1)?;
                let handle_ptr = args.nth_checked::<u32>(1)?;
                let handle_count = args.nth_checked::<u32>(1)?;

                let mut links = Vec::new();

                for offset in (0..handle_count).map(|x| x*4) {
                    let h: u32 = self.memory.get_value(handle_ptr + offset).map_err(|_| MemoryAccessOutOfBounds)?;

                    let key = self.handles
                    .get(h.try_into().expect("could not convert a u32 to a usize?"))
                    .ok_or(InvalidHandleError(h))?
                    .as_key().ok_or(InvalidHandleError(h))?;

                    links.push(*key);
                }

                let data = self
                    .memory
                    .get(src, len as usize)
                    .map_err(|_| MemoryAccessOutOfBounds)?;

                let key = self.journal.cas_put(distcomp::CASObj {
                    data,
                    links,
                });

                let handle: u32 = self.handles.insert(Handle::Key(key)).expect("failed to insert handle").try_into().expect("could nto convert a handle to a u32");

                Ok(Some(handle.into()))
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

                std::io::stdout().flush().expect("failed to flush stdout");

                Ok(Some(len.into()))
            }
            6 => {
                let handle = args.nth_checked::<u32>(0)?;
                let dest_addr = args.nth_checked::<u32>(1)?;
                let len = args.nth_checked::<u32>(2)?;
                let offset = args.nth_checked::<u32>(3)?;

                let data = self.handles.get(handle as usize).expect("failed to get handle").as_data().expect("invalid handle type");

                let start = u32::min(data.len() as u32, offset);
                let stop = u32::min(data.len() as u32, offset + len);

                let data_sliced = &data[start as usize ..stop as usize];

                assert!(data_sliced.len() <= len as usize);

                self.memory.set(dest_addr, data_sliced).expect("failed to write memory");
                
                Ok(Some((data_sliced.len() as u32).into()))
            }
            7 => {
                use wasmi::LittleEndianConvert;

                let handle = args.nth_checked::<u32>(0)?;

                let data = self.handles.get(handle as usize).expect("failed to get handle").as_key().expect("invalid handle type");

                let mut buf = Vec::new();

                let links = self.journal.cas_get(*data).expect("failed to get object").links;

                for link in links {
                    let handle = self.handles.insert(Handle::Key(link)).expect("failed to insert handle") as u32;
                    let mut handle_le = [0u8; 4];

                    handle.into_little_endian(&mut handle_le);

                    buf.extend_from_slice(&handle_le)
                }

                Ok(Some(I32(self.handles.insert(Handle::Data(buf)).expect("failed to insert handle").try_into().expect("failed to convert usize to handle"))))
            }
            8 => {
                let handle = args.nth_checked::<u32>(0)?;

                self.handles.release(handle as usize);

                Ok(None)
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
                    wasmi::Signature::new(&[][..], Some(I32)),
                    2,
                ));
            }
            "cas_get" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32][..], Some(I32)),
                    3,
                ));
            }
            "cas_put" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32, I32, I32, I32][..], Some(I32)),
                    4,
                ));
            }
            "output" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32, I32][..], Some(I32)),
                    5,
                ));
            }
            "read" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32, I32, I32, I32][..], Some(I32)),
                    6,
                ));
            }
            "cas_get_links" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32][..], Some(I32)),
                    7,
                ));
            }
            "handle_release" => {
                return Ok(wasmi::FuncInstance::alloc_host(
                    wasmi::Signature::new(&[I32][..], None),
                    8,
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

    let handles = Handles::default();

    let mut externals = HostExternals {
        appid,
        journal,
        memory,
        handles,
    };

    instance
        .invoke_export("main", &[], &mut externals)
        .expect("failed to execute export");
}

fn main() {
    better_panic::install();

    let journal = SqliteJournal::new("sqlite.db");

    let appid = ApplicationId(Uuid::parse_str("f524b42d-7108-4489-8c84-988462634d39").unwrap());

    func_main(appid, Box::new(journal));
}
