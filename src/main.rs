use rich_engine::startup;

fn main() {
    let info = rich_editor::get_external_startup_info();
    startup(info);
}
