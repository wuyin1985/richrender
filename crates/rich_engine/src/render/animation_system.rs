use bevy::prelude::*;
use bevy::tasks::ComputeTaskPool;
use crate::{Buffer, RenderContext};
use crate::render::animation::Animations;
use crate::render::node::Nodes;
use crate::render::skin::Skin;
use ash::vk;

pub const MAX_JOINTS_PER_MESH: usize = 512;

type SkinJointsUbo = [Mat4; MAX_JOINTS_PER_MESH];

pub struct AnimationRuntimeInit {}

pub struct AnimationWithNodes {
    pub animations: Animations,
    pub nodes: Nodes,
    pub skins: Vec<Skin>,
    pub skin_buffer: Buffer,
    pub skin_descriptor_set: vk::DescriptorSet,
}

impl AnimationWithNodes {
    pub fn destroy(&mut self, context: &RenderContext) {
        self.skin_buffer.destroy(context);
    }

    pub fn create(context: &mut RenderContext, animations: Animations, nodes: Nodes, skins: Vec<Skin>) -> Self {
        let skin_count = skins.len();
        assert!(skin_count > 0, "zero skin size");
        let elem_size = context.get_ubo_alignment::<SkinJointsUbo>();
        let skin_buffer = Buffer::create_host_visible_buffer_with_size(context, vk::BufferUsageFlags::UNIFORM_BUFFER,
                                                                       (elem_size * (skin_count as u32)) as _);

        let skin_descriptor_set = {
            let layouts = [context.skin_buffer_mgr.descriptor_set_layout];
            let allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(context.descriptor_pool)
                .set_layouts(&layouts);
            let set = unsafe {
                context
                    .device
                    .allocate_descriptor_sets(&allocate_info)
                    .unwrap()[0]
            };

            let descriptor_buffer_info = [vk::DescriptorBufferInfo::builder()
                .buffer(skin_buffer.buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE)
                .build()];

            let descriptor_write_info = vk::WriteDescriptorSet::builder()
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .dst_set(set)
                .dst_binding(0)
                .buffer_info(&descriptor_buffer_info)
                .build();

            unsafe {
                context.device.update_descriptor_sets(&[descriptor_write_info], &[]);
            }

            set
        };

        AnimationWithNodes { animations, nodes, skins, skin_buffer, skin_descriptor_set }
    }
}


pub struct AnimationRuntime {
    pub data: Option<AnimationWithNodes>,
}

impl Default for AnimationRuntime {
    fn default() -> Self {
        Self {
            data: None,
        }
    }
}

impl AnimationRuntime {
    pub fn update_animation_nodes(&mut self, delta_time: f32, commands: &mut AnimCommands) -> bool {
        if let Some(anim_nodes) = self.data.as_mut() {
            for command in &commands.data {
                match command {
                    AnimCommand::Play { index } => {
                        anim_nodes.animations.set_current(*index as _);
                        anim_nodes.animations.play();
                    }
                    AnimCommand::Stop => {
                        anim_nodes.animations.stop();
                    }
                }
            }
            commands.data.clear();

            if anim_nodes.animations.update(&mut anim_nodes.nodes, delta_time) {
                anim_nodes.nodes.transform(None);
                anim_nodes.nodes
                    .get_skins_transform()
                    .iter()
                    .for_each(|(index, transform)| {
                        let skin = &mut anim_nodes.skins[*index];
                        skin.compute_joints_matrices(*transform, &anim_nodes.nodes.nodes());
                    });

                return true;
            }
        }

        return false;
    }

    pub fn update_buffer(&mut self, context: &RenderContext) {
        if let Some(anim_nodes) = self.data.as_mut() {
            let skins = &anim_nodes.skins;
            let mut skin_matrices = vec![[Mat4::IDENTITY; MAX_JOINTS_PER_MESH]; skins.len()];
            for (skin_idx, skin) in skins.iter().enumerate() {
                let ms = &mut skin_matrices[skin_idx];
                for (joint_idx, joint) in skin.joints().iter().enumerate() {
                    let m = joint.matrix();
                    ms[joint_idx] = m;
                }
            }
            unsafe {
                let buffer = &mut anim_nodes.skin_buffer;
                let ptr = buffer.map_memory(context);
                let elem_size = context.get_ubo_alignment::<SkinJointsUbo>();
                Buffer::mem_copy_aligned(ptr, elem_size as _, &skin_matrices);
            }
        }
    }
}

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

pub fn update_animation_system(
    pool: Res<ComputeTaskPool>,
    time: Res<Time>,
    mut query: Query<(&mut AnimationRuntime, &mut AnimCommands)>) {
    query.par_for_each_mut(&pool, 32, |(mut runtime, mut commands)| {
        runtime.update_animation_nodes(time.delta_seconds(), &mut commands);
    });
}
