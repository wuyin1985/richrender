use bevy::reflect::TypeUuid;
use crate::game::actor::{ChannelAbility, ShootAbility};
use crate::game::table_data::TableDataItem;
use crate::prelude::*;

#[derive(serde::Deserialize)]
#[derive(serde::Serialize)]
#[derive(TypeUuid)]
#[uuid = "9a852db2-3eb7-4c91-99ae-ec1ea92f2877"]
pub struct AttackerConfig {
    pub name: String,
    pub ability_holders: Vec<AttackAbilityHolderConfig>,
}

impl TableDataItem for AttackerConfig {
    fn get_name(&self) -> &str {
        &self.name
    }
}

#[derive(serde::Deserialize)]
#[derive(serde::Serialize)]
pub struct AttackAbilityHolderConfig {
    pub ability: AbilityConfig,
}

#[derive(serde::Deserialize)]
#[derive(serde::Serialize)]
pub enum AbilityConfig {
    Shoot(ShootAbility),
    Channel(ChannelAbility),
}
