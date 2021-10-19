use rich_engine::startup;
use std::env;

fn main() {
    let info = rich_editor::get_external_startup_info();
    let args: Vec<String> = env::args().collect();
    startup(info, args);
}
