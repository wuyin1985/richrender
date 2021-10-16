use std::collections::HashMap;

pub const LOCATION_IN_POS: u32 = 0;
pub const LOCATION_IN_NORMAL: u32 = 1;
pub const LOCATION_IN_TEX_COORD: u32 = 2;
pub const LOCATION_IN_WEIGHTS: u32 = 3;
pub const LOCATION_IN_JOINTS: u32 = 4;

lazy_static! {
    static ref DEFINE_MAP: HashMap<u32,&'static str> = [
        (LOCATION_IN_TEX_COORD, "IN_TEX_COORD"),
        (LOCATION_IN_NORMAL, "IN_NORMAL"),
    ].iter().copied().collect();
}

pub fn get_shader_define_name(location: u32) -> Option<&'static str> {
    DEFINE_MAP.get(&location).and_then(|s| { Some(*s) })
}
