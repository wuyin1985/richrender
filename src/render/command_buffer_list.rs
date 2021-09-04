﻿use ash::vk;
use crate::render::device_mgr::DeviceMgr;
use std::convert::TryInto;

pub struct CommandBufferList {
    command_pool: vk::CommandPool,
    commands: Vec<vk::CommandBuffer>,
}

impl CommandBufferList {
    pub fn destroy(&mut self, device_mgr: &DeviceMgr) {
        unsafe {
            device_mgr.device.free_command_buffers(self.command_pool,&self.commands);
            device_mgr.device.destroy_command_pool(self.command_pool, None);
        }
    }

    pub fn create(count: u32, device_mgr: &DeviceMgr) -> Self {
        unsafe {
            let pool_ci = vk::CommandPoolCreateInfo {
                queue_family_index: device_mgr.graphics_queue_family_index,
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER | vk::CommandPoolCreateFlags::TRANSIENT,
                ..Default::default()
            };
            let command_pool = device_mgr.device.create_command_pool(&pool_ci, None).unwrap();
            let command_ci = vk::CommandBufferAllocateInfo {
                command_pool,
                command_buffer_count: count,
                level: vk::CommandBufferLevel::PRIMARY,
                ..Default::default()
            };
            let commands = device_mgr.device.allocate_command_buffers(&command_ci).unwrap();

            CommandBufferList {
                command_pool,
                commands,
            }
        }
    }

    pub fn get_command_buffer(&self, index: usize) -> vk::CommandBuffer {
        self.commands[index]
    }
}