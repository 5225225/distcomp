use distcomp::data_structures::kvs::KeyValueStore;
use distcomp::{ApplicationId, Journal, SqliteJournal};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasmi::{ImportsBuilder, ModuleInstance, NopExternals, RuntimeValue};

#[derive(Serialize, Deserialize, Default, Debug, Hash, Clone)]
struct Password {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PasswordManagerData {
    passwords: KeyValueStore<String, Password>,
}

fn do_foo(journal: &SqliteJournal, appid: ApplicationId, num: i32) {
    let state = journal.get_state(appid);

    let mut pwmandata;

    if let Some(s) = state.map(|x| x.0) {
        pwmandata = s;
    } else {
        pwmandata = PasswordManagerData {
            passwords: KeyValueStore::new(),
        };
    }

    pwmandata.passwords.insert(
        journal,
        format!("old_meme #{}", num + 10000),
        Password {
            username: format!("AzureDiamond-{}", num + 5000),
            password: format!("hunter{}", num),
        },
    );

    journal.update_state(&pwmandata, appid);
}

fn func_main() {
    // Parse WAT (WebAssembly Text format) into wasm bytecode.
    let wasm_binary: Vec<u8> = wabt::wat2wasm(
        r#"
            (module
                (func (export "test") (result i32)
                    i32.const 1337
                )
            )
            "#,
    )
    .expect("failed to parse wat");

    // Load wasm binary and prepare it for instantiation.
    let module = wasmi::Module::from_buffer(&wasm_binary).expect("failed to load wasm");

    // Instantiate a module with empty imports and
    // assert that there is no `start` function.
    let instance = ModuleInstance::new(&module, &ImportsBuilder::default())
        .expect("failed to instantiate wasm module")
        .assert_no_start();

    // Finally, invoke the exported function "test" with no parameters
    // and empty external function executor.
    assert_eq!(
        instance
            .invoke_export("test", &[], &mut NopExternals,)
            .expect("failed to execute export"),
        Some(RuntimeValue::I32(1337)),
    );
}

fn main() {
    better_panic::install();
    let journal = SqliteJournal::new("sqlite.db");

    let appid = ApplicationId(Uuid::parse_str("f524b42d-7108-4489-8c84-988462634d39").unwrap());

    func_main();
}
