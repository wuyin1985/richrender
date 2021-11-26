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

typedef struct Test_T22 *testT;

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

void get_image_and_view(ShareTexture &texture, vk::Image &image, vk::ImageView &view) {
    auto imageHandle = (VkImage) texture.image;
    auto imageViewHandle = (VkImageView) texture.view;

    image = vk::Image(imageHandle);
    view = vk::ImageView(imageViewHandle);
}

int StartupWithExternalVulkan(uint64_t vk_device, uint64_t vk_phy_device,
                              uint64_t vk_queue, uint64_t vk_command_pool,
                              ShareTexture color, ShareTexture depth) {

    auto vkQueueHandle = (VkQueue) vk_queue;
    auto vkDeviceHandle = (VkDevice) vk_device;
    auto vkPhyDeviceHandle = (VkPhysicalDevice) vk_phy_device;
    //auto vkCommandPoolHandle = (VkCommandPool) vk_command_pool;

    auto vkQueue = vk::Queue(vkQueueHandle);
    auto vkDevice = vk::Device(vkDeviceHandle);
    auto vkPhyDevice = vk::PhysicalDevice(vkPhyDeviceHandle);
    //auto vkCommandPool = vk::CommandPool(vkCommandPoolHandle);

    vk::CommandPoolCreateInfo cmdPoolInfo;
    cmdPoolInfo.queueFamilyIndex = 0;
    cmdPoolInfo.flags = vk::CommandPoolCreateFlagBits::eResetCommandBuffer;
    auto vkCommandPool = vkDevice.createCommandPool(cmdPoolInfo);

    auto queueFamilyProperties = vkPhyDevice.getQueueFamilyProperties();
    int graphicsQueueInd = -1;

    for (size_t i = 0; i < queueFamilyProperties.size(); i++) {
        auto &queueProp = queueFamilyProperties[i];
        if (queueProp.queueFlags &
            vk::QueueFlagBits::eGraphics /* && vkPhysicalDevice.getSurfaceSupportKHR(i, surface_)*/) {
            graphicsQueueInd = static_cast<int32_t>(i);
            break;
        }
    }

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
            vkCommandPool,
            vkPhyDevice,
            1,
            addCommand,
            nullptr,
            nullptr);

    auto colorTexture = new LLGI::TextureVulkan();
    auto colorSize = LLGI::Vec2I(color.width, color.height);

    {
        vk::Image image;
        vk::ImageView view;
        get_image_and_view(color, image, view);

        colorTexture->InitializeAsScreen(
                image,
                view,
                static_cast<vk::Format>(color.format),
                colorSize);

        colorTexture->SetType(LLGI::TextureType::Render);
    }

    auto depthTexture = new LLGI::TextureVulkan();

    auto depthSize = LLGI::Vec2I(depth.width, depth.height);

    {

        vk::Image image;
        vk::ImageView view;
        get_image_and_view(depth, image, view);

        depthTexture->InitializeAsDepthExternal(
                image,
                view,
                static_cast<vk::Format>(depth.format),
                depthSize);

        depthTexture->SetType(LLGI::TextureType::Depth);
    }

    auto renderPass = graphics->CreateRenderPass(colorTexture, nullptr, depthTexture, nullptr);
    Startup(graphics, renderPass);
    return 0;
}

int32_t TestCall(int32_t input) {
    return input + 1;
}