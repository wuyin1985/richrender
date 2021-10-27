use std::collections::HashMap;
use rich_engine::startup;
use std::env;
use std::pin::Pin;
use std::ptr::NonNull;

fn main() {
    let info = rich_editor::get_external_startup_info();
    let args: Vec<String> = env::args().collect();
    startup(info, args);
}

