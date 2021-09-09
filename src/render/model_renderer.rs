use crate::render::model::Model;
use ash::vk;

pub struct ModelRenderer {
    model: Model,
    pipeline: vk::Pipeline,
}