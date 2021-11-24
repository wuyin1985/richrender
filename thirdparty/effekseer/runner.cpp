//
// Created by yin.wu on 2021/11/23.
//

#include "export.hpp"
#include <Effekseer.h>
#include <EffekseerRendererVulkan.h>
#include <LLGI.Graphics.h>
#include <LLGI.Platform.h>
#include <Utils/LLGI.CommandListPool.h>
#include <Vulkan/LLGI.CommandListVulkan.h>
#include <Vulkan/LLGI.GraphicsVulkan.h>
#include <Vulkan/LLGI.TextureVulkan.h>

void RunWithPlatform() {
    int32_t windowWidth = 1280;
    int32_t windowHeight = 720;

    LLGI::PlatformParameter platformParam{};
    platformParam.Device = LLGI::DeviceType::Vulkan;
    platformParam.WaitVSync = true;
    auto window = std::shared_ptr<LLGI::Window>(
            LLGI::CreateWindow("Vulkan", LLGI::Vec2I(windowWidth, windowHeight)));

    auto platform = LLGI::CreateSharedPtr(
            LLGI::CreatePlatform(platformParam, window.get()));

    auto graphics = LLGI::CreateSharedPtr(platform->CreateGraphics());

    Startup(graphics.get(), nullptr);

    LLGI::Color8 color;
    color.R = 0;
    color.G = 0;
    color.B = 0;
    color.A = 255;

    while (platform->NewFrame()) {
        auto renderPass = platform->GetCurrentScreen(color, true, true);
        UpdateFrame(renderPass);
        platform->Present();
    }

    Shutdown();
}

int StartupWithExternalVulkan(void *vk_device, void *vk_phy_device,
                              void *vk_queue, void *vk_command_pool,
                              ShareTexture color, ShareTexture depth) {

    auto vkQueue = *reinterpret_cast<const vk::Queue *>(vk_queue);
    auto vkDevice = *reinterpret_cast<const vk::Device *>(vk_device);

    auto addCommand = [vkQueue](vk::CommandBuffer commandBuffer, vk::Fence fence) -> void {
        std::array<vk::SubmitInfo, 1> copySubmitInfos;
        copySubmitInfos[0].commandBufferCount = 1;
        copySubmitInfos[0].pCommandBuffers = &commandBuffer;
        vkQueue.submit(static_cast<uint32_t>(copySubmitInfos.size()), copySubmitInfos.data(),
                       fence);
    };


    auto graphics = new LLGI::GraphicsVulkan(
            vkDevice,
            vkQueue,
            *reinterpret_cast<const vk::CommandPool *>(vk_command_pool),
            *reinterpret_cast<const vk::PhysicalDevice *>(vk_phy_device),
            1,
            addCommand,
            nullptr,
            nullptr);

    auto colorTexture = new LLGI::TextureVulkan();
    auto colorSize = LLGI::Vec2I(color.width, color.height);
    colorTexture->InitializeAsScreen(
            *reinterpret_cast<vk::Image *>(color.image),
            *reinterpret_cast<vk::ImageView *>(color.view),
            static_cast<vk::Format>(color.format),
            colorSize);

    auto depthTexture = new LLGI::TextureVulkan();
    auto depthSize = LLGI::Vec2I(depth.width, depth.height);
    depthTexture->InitializeAsScreen(
            *reinterpret_cast<vk::Image *>(depth.image),
            *reinterpret_cast<vk::ImageView *>(depth.view),
            static_cast<vk::Format>(depth.format),
            depthSize);

    auto renderPass = new LLGI::RenderPassVulkan(nullptr, vkDevice, nullptr);

    std::array<LLGI::TextureVulkan *, 1> textures;
    textures[0] = colorTexture;
    renderPass->Initialize(const_cast<const LLGI::TextureVulkan **>(textures.data()), 1,
                           depthTexture, nullptr, nullptr);


    Startup(graphics, renderPass);
    return 0;
}
