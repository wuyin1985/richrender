use bevy::prelude::*;
use bevy::tasks::ComputeTaskPool;
use crate::{Buffer, RenderContext};
use crate::render::animation::Animations;
use crate::render::node::Nodes;
use crate::render::skin::Skin;
use ash::vk;

pub enum AnimCommand {
    Play { index: u32 },
    Stop,
}

pub struct AnimCommands {
    pub data: Vec<AnimCommand>,
}

impl AnimCommands {
    pub fn create() -> Self {
        Self { data: Vec::new() }
    }

    pub fn create_with_commands(commands: Vec<AnimCommand>) -> Self {
        Self { data: commands }
    }

    pub fn push(&mut self, command: AnimCommand) -> &mut Self {
        self.data.push(command);
        self
    }
}

