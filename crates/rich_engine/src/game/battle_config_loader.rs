use std::fs::File;
use bevy::prelude::*;
use std::io;
use std::io::Read;
use crate::game::config_define::AttackerConfig;
use crate::game::table_data::{TableData, TableDataItem};


fn load_table<T>(ron_name: &str) -> TableData<T> where T: TableDataItem {
    let mut data = vec![];
    let mut f = File::open(format!("./assets/config/{}.ron", ron_name))
        .unwrap_or_else(|_| panic!("failed to load config {}", ron_name));
    match f.read_to_end(&mut data) {
        Ok(_) => {}
        Err(e) => {
            panic!("error {} in load config {}", e, ron_name);
        }
    };

    TableData::load_from_bytes(&data)
}


pub fn load_battle_configs(app: &mut AppBuilder) {
    let attacker = load_table::<AttackerConfig>("attacker");
    app.insert_resource(attacker);
}