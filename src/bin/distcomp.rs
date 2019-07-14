use distcomp::data_structures::kvs::KeyValueStore;
use distcomp::{ApplicationId, Journal, SqliteJournal};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

fn main() {
    better_panic::install();
    let journal = SqliteJournal::new("sqlite.db");

    let appid = ApplicationId(Uuid::parse_str("f524b42d-7108-4489-8c84-988462634d39").unwrap());

    for i in 0..1000 {
        do_foo(&journal, appid, dbg!(i));
    }
}
