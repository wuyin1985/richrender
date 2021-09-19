use bevy::utils::HashMap;
use ash::vk;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::render::render_context::RenderContext;
use std::path::Path;
use std::io::Cursor;
use bevy::prelude::*;
use bevy::reflect::List;

pub struct ShaderCollection {
    modules: HashMap<u64, vk::ShaderModule>,
}

impl Default for ShaderCollection {
    fn default() -> Self {
        Self::create()
    }
}

fn load_from_file(path: &str) -> Cursor<Vec<u8>> {
    use std::fs::File;
    use std::io::Read;
    let mut buf = Vec::new();
    let fullpath = &Path::new(path);
    let mut file = File::open(&fullpath).unwrap();
    file.read_to_end(&mut buf).unwrap();
    Cursor::new(buf)
}


impl ShaderCollection {
    pub fn create() -> Self {
        ShaderCollection { modules: HashMap::default() }
    }

    pub fn destroy(&mut self, context: &mut RenderContext) {
        let map = std::mem::take(&mut self.modules);
        for (_, shader) in map {
            unsafe {
                context.device.destroy_shader_module(shader, None);
            }
        }
    }

    pub fn create_shader(&mut self, device: &ash::Device, name: &str, defines: &[&str]) -> vk::ShaderModule {
        use std::process::Command;

        let mut s = DefaultHasher::new();
        name.hash(&mut s);
        for d in defines {
            d.hash(&mut s);
        }
        let id = s.finish();

        if let Some(sd) = self.modules.get(&id) {
            return *sd;
        }

        let source_path = format!("assets/shaders/{}.glsl", name);
        let out_path = format!("assets/spv/temp/{}.spv", id.to_string());

        let mut args = vec!["/C", "glslc.exe", &source_path, "-o", &out_path];
        let define_cmd = defines.iter().map(|d| format!("-D{}", d)).collect::<Vec<String>>();
        for d in &define_cmd {
            args.push(d.as_str());
        }

        let output = Command::new("cmd").args(&args)
            .output().expect("failed to execute command");

        for out in String::from_utf8(output.stdout).iter() {
            info!("compile info: {}", out);
        }

        let mut has_error = false;

        for out in String::from_utf8(output.stderr).iter() {
            error!("compile info: {}", out);
            //has_error = true;
        }

        // if has_error {
        //     panic!("compile shader panic");
        // }


        let mut cursor = load_from_file(&out_path);
        let res = ash::util::read_spv(&mut cursor).expect(format!("failed to read spv {}", source_path).as_str());
        let create_info = vk::ShaderModuleCreateInfo::builder().code(res.as_slice()).build();
        let sd = unsafe { device.create_shader_module(&create_info, None).unwrap() };
        self.modules.insert(id, sd);
        sd
    }
}