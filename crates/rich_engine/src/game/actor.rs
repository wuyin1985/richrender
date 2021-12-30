pub struct Attacker {}

pub struct AttackAbilityHolder {}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ShootAbility {
    pub cd: f32,
    pub reload_time: f32,
    pub magazine: u32,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ChannelAbility {
    pub total_value: f32,
    pub value_cost_speed: f32,
}


pub fn attack_system() {}
