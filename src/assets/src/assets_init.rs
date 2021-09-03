use bevy_ecs::prelude::*;
use assets_manager::*;
use serde::Deserialize;

// The struct you want to load
#[derive(Deserialize)]
struct Monster {
    name: String,
    description: String,
    health: u32,
}

impl Asset for Monster {
    const EXTENSION: &'static str = "ron";
    type Loader = assets_manager::loader::RonLoader;
}

pub fn init(world: &mut World, path: &str) {
    let cache = AssetCache::new(path).unwrap();
    let k = cache.load::<Monster>("point").unwrap();
    let s = k.read();
    println!("point {} {}", s.name, s.health);
}