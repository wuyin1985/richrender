use bevy::{
    prelude::*,
    asset::{AssetLoader, LoadContext, LoadedAsset},
    reflect::TypeUuid,
    utils::BoxedFuture,
};
use crate::render::animation::Animations;
use crate::render::node::Nodes;
use crate::render::skin::Skin;
use crate::{Buffer, RenderContext};
use ash::vk;

#[derive(Debug)]
pub enum GltfData {
    Parsed,
    Raw {
        document: gltf::Document,
        buffers: Vec<gltf::buffer::Data>,
        images: Vec<gltf::image::Data>,
    },
}

pub const MAX_JOINTS_PER_MESH: usize = 512;

type SkinJointsUbo = [Mat4; MAX_JOINTS_PER_MESH];

pub struct GltfAnimationRuntimeInit {}

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


pub struct GltfAnimationRuntime {
    pub data: Option<AnimationWithNodes>,
}

impl Default for GltfAnimationRuntime {
    fn default() -> Self {
        Self {
            data: None,
        }
    }
}

impl GltfAnimationRuntime {
    pub fn update_animation_nodes(&mut self, delta_time: f32) -> bool {
        if let Some(anim_nodes) = self.data.as_mut() {
            if anim_nodes.animations.update(&mut anim_nodes.nodes, delta_time) {
                //anim_nodes.nodes.transform(None);
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
            let mut skin_matrices = vec![[Mat4::identity(); MAX_JOINTS_PER_MESH]; skins.len()];
            for (skin_idx, skin) in skins.iter().enumerate() {
                let ms = &mut skin_matrices[skin_idx];
                for (joint_idx, joint) in skin.joints().iter().enumerate() {
                    ms[joint_idx] = joint.matrix();
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


#[derive(Debug, TypeUuid)]
#[uuid = "f779f9ea-41cd-48ad-a553-0894d84a4be7"]
pub struct GltfAsset {
    data: GltfData,
}

impl GltfAsset {
    pub fn export(&self) -> (
        &gltf::Document,
        &Vec<gltf::buffer::Data>,
        &Vec<gltf::image::Data>,
    )
    {
        if let GltfData::Raw { document, buffers, images } = &self.data {
            return (&document, &buffers, &images);
        }

        panic!("not raw")
    }

    pub fn set_parsed(&mut self) {
        self.data = GltfData::Parsed {}
    }
}

#[derive(Default)]
pub struct GltfAssetLoader;

impl AssetLoader for GltfAssetLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let (document, buffers, images) = gltf::import_slice(bytes)?;
            let data = GltfAsset { data: GltfData::Raw { document, buffers, images } };
            load_context.set_default_asset(LoadedAsset::new(data));
            println!("load !! assets");
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }
}

