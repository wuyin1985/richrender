use bevy::app::AppBuilder;
use bevy::prelude::*;
use crate::game::battle_config_loader::load_battle_configs;
use crate::{Commands, Plugin};
use crate::game::actor::{AttackAbilityHolder, Attacker};
use crate::game::config_define::{AbilityConfig, AttackerConfig};
use crate::game::table_data::TableData;

fn game_startup(attack_table: Res<TableData<AttackerConfig>>,
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