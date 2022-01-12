mod map;
mod actor;
mod config_define;
mod table_data;
mod battle_config_loader;
mod game_plugin;
mod proto;

pub use game_plugin::GamePlugin;


#[cfg(test)]
mod test {
    use std::env;
    use std::fs::File;
    use std::io::Read;
    use quick_protobuf::{BytesReader, MessageRead};
    use super::proto::proto;

    #[test]
    fn test_read_map_config() {
        let proto_path = format!("d:/learn/richrender/assets/config/map/Map_byte.bin");
        let mut f = File::open(&proto_path)
            .unwrap_or_else(|_| panic!("failed to load map config {}", &proto_path));
        let mut data = vec![];
        match f.read_to_end(&mut data) {
            Ok(_) => {
                let mut reader = BytesReader::from_bytes(&data);
                match proto::Map::from_reader(&mut reader, &data) {
                    Ok(map) => {
                        println!("read => {:?}", map);
                    }
                    Err(e) => {
                        panic!("failed to decode map config {} {}", &proto_path, e)
                    }
                }
            }
            Err(e) => {
                panic!("error {} in load map config {}", e, &proto_path);
            }
        };
    }
}