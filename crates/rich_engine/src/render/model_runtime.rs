use std::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::ComputeTaskPool;
use crate::{AnimCommand, AnimCommands, Buffer, GltfAsset, RenderContext, RenderRunner};
use crate::render::animation::{Animations, NodesKeyFrame};
use crate::render::node::{Node, Nodes};
use crate::render::skin::{Joint, Skin};
use ash::vk;
use crate::core::destroy::Destroy;

pub const MAX_JOINTS_PER_MESH: usize = 512;

type SkinJointsUbo = [Mat4; MAX_JOINTS_PER_MESH];

pub struct ModelRuntime {
    nodes: Vec<ModelNode>,
    named_nodes: HashMap<u64, usize>,
}

impl ModelRuntime {
    pub fn get_nodes(&self) -> &Vec<ModelNode> {
        &self.nodes
    }
}

pub struct ModelNode {
    pub entity: Entity,
    pub node: Node,
}

pub struct ModelSkins {
    pub skins: Vec<Skin>,
    pub skin_buffer: Buffer,
    pub skin_buffer_element_size: u32,
    pub skin_descriptor_set: vk::DescriptorSet,
    pub valid: bool,
}

impl ModelSkins {
    pub fn destroy(&mut self, context: &RenderContext) {
        assert!(self.valid, "the model skins has already destroy");
        self.valid = false;
        self.skin_buffer.destroy(context);
        unsafe {
            context.device.free_descriptor_sets(context.descriptor_pool, &[self.skin_descriptor_set]);
        }
    }
}

pub struct ModelJointRef {
    model_entity: Entity,
    skin_entity: Entity,
    skin_idx: usize,
    joint_idx: usize,
    joint: Joint,
}

fn create_node_with_children(builder: &mut ChildBuilder, nodes: &[Node],
                             idx: usize, entity_map: &mut HashMap<usize, Entity>) {
    let node = &nodes[idx];
    let mut cmd = builder.spawn();
    cmd.insert(Transform {
        translation: node.get_local_position(),
        scale: node.get_local_scale(),
        rotation: node.get_local_rotation(),
    }).insert(GlobalTransform::identity());
    entity_map.insert(idx, cmd.id());

    cmd.with_children(|child_builder| {
        for child_idx in node.get_children() {
            create_node_with_children(child_builder, nodes, *child_idx, entity_map);
        }
    });
}

pub fn init_model_runtime_system(
    mut commands: Commands,
    mut runner: Option<ResMut<RenderRunner>>,
    mut query: Query<(Entity, &Handle<GltfAsset>), Without<ModelRuntime>>) {
    if let Some(mut runner) = runner {
        let context = &mut runner.context;
        for (entity, handle) in query.iter_mut() {
            let nodes_and_skins: Option<(Nodes, Vec<Skin>, Option<Animations>)> = {
                if let Some(model_renderer) = context.get_model(handle) {
                    let model = model_renderer.get_model();
                    Some((model.nodes().clone(), model.get_skins().clone(), model.clone_animations()))
                } else {
                    None
                }
            };

            if let Some((nodes, skins, animations)) = nodes_and_skins {
                if skins.len() > 1 {
                    //todo 多重skin需要创建多个set
                    panic!("multi skin not supported ")
                }
                //create runtime
                let mut named_nodes: HashMap<u64, usize> = HashMap::new();
                let mut entity_map: HashMap<usize, Entity> = HashMap::new();
                commands.entity(entity).with_children(|builder| {
                    for idx in nodes.get_roots() {
                        create_node_with_children(builder, nodes.nodes(), *idx, &mut entity_map);
                    }
                });

                let model_nodes = nodes.nodes().iter().enumerate().map(|(idx, node)| {
                    if let Some(name) = node.name() {
                        named_nodes.insert(name, idx);
                    }
                    let node_entity = *entity_map.get(&idx).unwrap();
                    ModelNode { entity: node_entity, node: node.clone() }
                }).collect::<Vec<_>>();

                //create skin
                if skins.len() > 0 {
                    // add skin joint ref to node entity
                    for (node_idx, node) in nodes.nodes().iter().enumerate() {
                        if let Some(skin_idx) = node.skin_index() {
                            let skin = &skins[skin_idx];
                            for (joint_idx, joint) in skin.joints().iter().enumerate() {
                                let model_node = &model_nodes[joint.get_node_idx()];
                                commands.entity(model_node.entity).insert(
                                    ModelJointRef {
                                        skin_entity: model_nodes[node_idx].entity,
                                        model_entity: entity,
                                        skin_idx,
                                        joint_idx,
                                        joint: joint.clone(),
                                    });
                            }
                        }
                    }

                    let model_skins = create_model_skins(context, skins);
                    commands.entity(entity).insert(model_skins);
                }

                let mut cmd = commands.entity(entity);
                cmd.insert(ModelRuntime { nodes: model_nodes, named_nodes });
                if let Some(anim) = animations {
                    cmd.insert(anim);
                }
            }
        }
    }
}

pub fn destroy_model_skins_system(
    mut runner: Option<ResMut<RenderRunner>>,
    mut query: Query<(&mut ModelSkins), With<Destroy>>,
)
{
    if let Some(mut runner) = runner {
        for mut skins in query.iter_mut() {
            skins.destroy(&mut runner.context);
            info!("remove skins");
        }
    }
}

fn create_model_skins(context: &mut RenderContext, skins: Vec<Skin>) -> ModelSkins {
    let skin_count = skins.len();
    let elem_size = context.get_ubo_alignment::<SkinJointsUbo>();
    let buffer_size = elem_size * (skin_count as u32);
    let skin_buffer = Buffer::create_host_visible_buffer_with_size(context, vk::BufferUsageFlags::UNIFORM_BUFFER,
                                                                   buffer_size as _);

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

    ModelSkins {
        skins,
        skin_buffer,
        skin_descriptor_set,
        skin_buffer_element_size: elem_size,
        valid: true,
    }
}

pub fn update_model_runtime_animation(
    time: Res<Time>,
    mut runtime_query: Query<(&ModelRuntime, &mut Animations, &mut AnimCommands)>,
    mut transform_query: Query<&mut Transform>,
)
{
    let delta_time = time.delta_seconds();
    for (runtime, mut animations, mut commands) in runtime_query.iter_mut() {
        for command in &commands.data {
            match command {
                AnimCommand::Play { index } => {
                    animations.set_current(*index as _);
                    animations.play();
                }
                AnimCommand::Stop => {
                    animations.stop();
                }
            }
        }
        commands.data.clear();

        if let Some(NodesKeyFrame(translations, rotations, scale)) = animations.update(delta_time) {
            translations.iter().for_each(|(node_index, translation)| {
                if let Ok(mut t) = transform_query.get_mut(runtime.nodes[*node_index].entity) {
                    t.translation = *translation;
                }
            });
            rotations.iter().for_each(|(node_index, rotation)| {
                if let Ok(mut t) = transform_query.get_mut(runtime.nodes[*node_index].entity) {
                    t.rotation = *rotation;
                }
            });
            scale.iter().for_each(|(node_index, scale)| {
                if let Ok(mut t) = transform_query.get_mut(runtime.nodes[*node_index].entity) {
                    t.scale = *scale;
                }
            });
        }
    };
}

pub fn update_skin_joint_matrix(
    mut skins_query: Query<(&ModelSkins)>,
    mut transform_query: Query<(&GlobalTransform)>,
    mut joint_query: Query<(Entity, &GlobalTransform, &ModelJointRef), Changed<GlobalTransform>>,
)
{
    for (entity, global_transform, joint_ref) in joint_query.iter() {
        if let Ok(skin_transform) = transform_query.get(joint_ref.skin_entity) {
            if let Ok(skins) = skins_query.get(joint_ref.model_entity) {
                unsafe {
                    let joint_matrix = joint_ref.joint.compute_matrix_2_skin(skin_transform.compute_matrix(), global_transform.compute_matrix());
                    let ptr = skins.skin_buffer.get_memory();
                    let offset = joint_ref.skin_idx * (skins.skin_buffer_element_size as usize) +
                        joint_ref.joint_idx * (std::mem::size_of::<Mat4>() as usize);

                    let p = ptr.offset(offset as _);
                    let mut m = p as *mut Mat4;
                    m.copy_from(&joint_matrix, 1);
                }
            }
        }
    }
}
