use std::path::Path;
use bevy::app::AppBuilder;
use bevy::prelude::*;
use crate::game::battle_config_loader::load_battle_configs;
use crate::{Commands, DisplayName, GltfAsset, Plugin};
use crate::game::actor::{AttackAbilityHolder, Attacker};
use crate::game::config_define::{AbilityConfig, AttackerConfig};
use crate::game::table_data::TableData;
use crate::prelude::CameraOpEvent;
use crate::render::MainLight;

fn game_startup(attack_table: Res<TableData<AttackerConfig>>,
                asset_server: ResMut<AssetServer>,
                mut main_light_query: Query<(&mut Transform), With<MainLight>>,
                mut camera_op_event_writer: EventWriter<CameraOpEvent>,
                mut command: Commands) {
    let tower_config = attack_table.index_by_str("main");

    command.spawn().insert(Attacker {}).with_children(|cb| {
        for holder_config in &tower_config.ability_holders {
            let mut cmd = cb.spawn();
            cmd.insert(AttackAbilityHolder {});
            match &holder_config.ability {
                AbilityConfig::Shoot(s) => {
                    cmd.insert(s.clone());
                }
                AbilityConfig::Channel(c) => {
                    cmd.insert(c.clone());
                }
            }
        };
    });

    //add terrain
    let terrain_handle: Handle<GltfAsset> = asset_server.load("gltf/Map_export.glb");

    command.spawn().insert(terrain_handle)
        .insert(Transform::from_translation(Vec3::new(50f32, 0f32, -50f32)))
        .insert(GlobalTransform::identity())
        .insert(DisplayName::from_str("terrain"));

    if let Ok((transform)) = &mut main_light_query.single_mut() {
        transform.translation = Vec3::new(-24f32, 19f32, -2f32);
        transform.rotation = Quat::from_euler(EulerRot::ZXY, -110.0f32.to_radians(), -81.401f32.to_radians(), -110.2f32.to_radians());
    }

    camera_op_event_writer.send(CameraOpEvent::ChangeTranslation(Vec3::new(-19.436, 25.267, 1.445f32)));
    camera_op_event_writer.send(
        CameraOpEvent::ChangeRotation(Quat::from_euler(EulerRot::ZXY, -42.279f32.to_radians(), -1.918f32.to_radians(), -87.892f32.to_radians())));
}


pub struct GamePlugin {}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        //load battle configs
        load_battle_configs(app);

        //game startup
        app.add_startup_system(game_startup.system());
    }
}