use distcomp::data_structures::kvs::KeyValueStore;
use distcomp::{ApplicationId, Journal, SqliteJournal};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasmi::{ImportsBuilder, ModuleInstance, RuntimeValue};

struct HostExternals<T: Journal> {
    appid: ApplicationId,
    journal: T,
}



impl<T: Journal> wasmi::Externals for HostExternals<T> {
   fn invoke_index(
        &mut self,
        index: usize,
        args: wasmi::RuntimeArgs,
    ) -> Result<Option<wasmi::RuntimeValue>, wasmi::Trap> {
        match index {
            1 => {
                println!("Hello, World!");

                Ok(None)
            }
            _ => panic!("Unimplemented function at {}", index),
        }
    }
}

struct Resolver {}

impl wasmi::ModuleImportResolver for Resolver {
    fn resolve_func(&self, field_name: &str, _signature: &wasmi::Signature) -> Result<wasmi::FuncRef, wasmi::Error> {

        match field_name {
            "hello_world" => {
               return Ok(wasmi::FuncInstance::alloc_host(wasmi::Signature::new(&[][..], None), 1));
            }
            _ => {
                return Err(wasmi::Error::Instantiation("Failed to resolve".to_string()));
            }
        }

        unimplemented!();
    }
}

fn func_main<T: Journal>(appid: ApplicationId, journal: T) {
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

    let mut externals = HostExternals{appid, journal};

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

    func_main(appid, journal);
}
