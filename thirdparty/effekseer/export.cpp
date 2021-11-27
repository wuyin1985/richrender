//
// Created by yin.wu on 2021/11/19.
//

#include <stdio.h>
#include "export.hpp"
#include <Effekseer.h>
#include <EffekseerRendererVulkan.h>

VkPhysicalDevice GetVkPhysicalDevice();

VkDevice GetVkDevice();

VkQueue GetVkQueue();

VkCommandPool GetVkCommandPool();


#include <LLGI.Graphics.h>
#include <LLGI.Platform.h>
#include <Utils/LLGI.CommandListPool.h>
#include <Vulkan/LLGI.CommandListVulkan.h>
#include <Vulkan/LLGI.GraphicsVulkan.h>
#include <Vulkan/LLGI.TextureVulkan.h>

struct ContextLLGI {
    LLGI::Graphics *graphics;
    LLGI::RenderPass *renderPass;
    std::shared_ptr<LLGI::SingleFrameMemoryPool> memoryPool;
    std::shared_ptr<LLGI::CommandListPool> commandListPool;
    Effekseer::RefPtr<EffekseerRenderer::CommandList> commandListEfk;
    Effekseer::RefPtr<EffekseerRenderer::Renderer> renderer;
    Effekseer::RefPtr<Effekseer::Manager> manager;
    Effekseer::RefPtr<Effekseer::Effect> effect;
    Effekseer::RefPtr<EffekseerRenderer::SingleFrameMemoryPool> sfMemoryPoolEfk;
    Effekseer::Handle handle;
    int32_t time;
};

std::shared_ptr<ContextLLGI> context;

VkPhysicalDevice GetVkPhysicalDevice() {
    return static_cast<VkPhysicalDevice>(static_cast<LLGI::GraphicsVulkan *>(context->graphics)->GetPysicalDevice());
}

VkDevice
GetVkDevice() { return static_cast<VkDevice>(static_cast<LLGI::GraphicsVulkan *>(context->graphics)->GetDevice()); }

VkQueue
GetVkQueue() { return static_cast<VkQueue>(static_cast<LLGI::GraphicsVulkan *>(context->graphics)->GetQueue()); }

VkCommandPool GetVkCommandPool() {
    return static_cast<VkCommandPool>(static_cast<LLGI::GraphicsVulkan *>(context->graphics)->GetCommandPool());
}

void Startup(LLGI::Graphics *vGraphic, int32_t swap_buffer_count,
             ::EffekseerRendererVulkan::RenderPassInformation &renderPassInfo) {
    context = std::make_shared<ContextLLGI>();
    context->graphics = vGraphic;
    context->time = 0;
    context->memoryPool = LLGI::CreateSharedPtr(
            context->graphics->CreateSingleFrameMemoryPool(1024 * 1024, 128));
    context->commandListPool = std::make_shared<LLGI::CommandListPool>(context->graphics,
                                                                       context->memoryPool.get(),
                                                                       swap_buffer_count);

    auto renderer = ::EffekseerRendererVulkan::Create(
            GetVkPhysicalDevice(), GetVkDevice(), GetVkQueue(), GetVkCommandPool(),
            swap_buffer_count, renderPassInfo, 8000);
    context->renderer = renderer;

    auto sfMemoryPoolEfk = EffekseerRenderer::CreateSingleFrameMemoryPool(
            renderer->GetGraphicsDevice());
    context->sfMemoryPoolEfk = sfMemoryPoolEfk;

    auto commandListEfk = EffekseerRenderer::CreateCommandList(renderer->GetGraphicsDevice(),
                                                               sfMemoryPoolEfk);

    context->commandListEfk = commandListEfk;

    // Create a manager of effects
    auto manager = ::Effekseer::Manager::Create(8000);

    // Sprcify rendering modules
    manager->SetSpriteRenderer(renderer->CreateSpriteRenderer());
    manager->SetRibbonRenderer(renderer->CreateRibbonRenderer());
    manager->SetRingRenderer(renderer->CreateRingRenderer());
    manager->SetTrackRenderer(renderer->CreateTrackRenderer());
    manager->SetModelRenderer(renderer->CreateModelRenderer());

    // Specify a texture, model, curve and material loader
    // It can be extended by yourself. It is loaded from a file on now.
    manager->SetTextureLoader(renderer->CreateTextureLoader());
    manager->SetModelLoader(renderer->CreateModelLoader());
    manager->SetMaterialLoader(renderer->CreateMaterialLoader());
    manager->SetCurveLoader(Effekseer::MakeRefPtr<Effekseer::CurveLoader>());

    context->manager = manager;

    // Specify a position of view
    auto g_position = ::Effekseer::Vector3D(10.0f, 5.0f, 20.0f);

    int32_t windowWidth = 1280;
    int32_t windowHeight = 720;

    // Specify a projection matrix
    renderer->SetProjectionMatrix(
            ::Effekseer::Matrix44().PerspectiveFovRH(90.0f / 180.0f * 3.14f,
                                                     (float) windowWidth / (float) windowHeight,
                                                     1.0f, 500.0f));

    // Specify a camera matrix
    renderer->SetCameraMatrix(
            ::Effekseer::Matrix44().LookAtRH(g_position,
                                             ::Effekseer::Vector3D(0.0f, 0.0f, 0.0f),
                                             ::Effekseer::Vector3D(0.0f, 1.0f, 0.0f)));

    // Load an effect
    auto effect = Effekseer::Effect::Create(manager, u"assets/test.efk", 1.0f, nullptr);
    context->effect = effect;
}

void Shutdown() {
    context->manager.Reset();

    context->renderer.Reset();

    context->graphics->WaitFinish();

    context = nullptr;
}


void UpdateFrame(void *vRenderPass, uint64_t externalCommandBufferHandle) {
    LLGI::RenderPass *renderPass = context->renderPass;
    if (vRenderPass != nullptr) {
        renderPass = reinterpret_cast<LLGI::RenderPass *>(vRenderPass);
    }

    context->memoryPool->NewFrame();

    auto commandList = context->commandListPool->Get();

    auto vulkanList = static_cast<LLGI::CommandListVulkan *>(commandList);

    if(externalCommandBufferHandle != 0)
    {
        auto handle = (VkCommandBuffer) externalCommandBufferHandle;
        auto vkCommandBuffer = vk::CommandBuffer(handle);
        vulkanList->SetExternalCommandBuffer(vkCommandBuffer);
    }

    commandList->Begin();

    commandList->BeginRenderPass(renderPass);

    context->sfMemoryPoolEfk->NewFrame();

    auto commandBuffer = static_cast<VkCommandBuffer>(vulkanList->GetCommandBuffer());

    EffekseerRendererVulkan::BeginCommandList(context->commandListEfk, commandBuffer);
    context->renderer->SetCommandList(context->commandListEfk);

    auto manager = context->manager;
    auto handle = context->handle;
    int32_t time = context->time;

    if (time % 120 == 0) {
        context->handle = manager->Play(context->effect, 0, 0, 0);
    }

    if (time % 120 == 119) {
        manager->StopEffect(handle);
    }

    manager->AddLocation(handle, ::Effekseer::Vector3D(0.2f, 0.0f, 0.0f));

    manager->Update();

    auto renderer = context->renderer;

    renderer->SetTime(time / 60.0f);

    renderer->BeginRendering();

    manager->Draw();

    renderer->EndRendering();

    renderer->SetCommandList(nullptr);

    EffekseerRendererVulkan::EndCommandList(context->commandListEfk);

    commandList->EndRenderPass();

    commandList->End();

    context->graphics->Execute(commandList);

    context->time = time + 1;
}

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

    ::EffekseerRendererVulkan::RenderPassInformation renderPassInfo;
    renderPassInfo.DoesPresentToScreen = true;
    renderPassInfo.RenderTextureCount = 1;
    renderPassInfo.RenderTextureFormats[0] = VK_FORMAT_B8G8R8A8_UNORM;
    renderPassInfo.DepthFormat = VK_FORMAT_D24_UNORM_S8_UINT;
    Startup(graphics.get(), 3, renderPassInfo);

    LLGI::Color8 color;
    color.R = 0;
    color.G = 0;
    color.B = 0;
    color.A = 255;

    while (platform->NewFrame()) {
        auto renderPass = platform->GetCurrentScreen(color, true, true);
        UpdateFrame(renderPass, 0);
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

uint64_t StartupWithExternalVulkan(uint64_t vk_device, uint64_t vk_phy_device, uint64_t vk_queue,
                                   uint64_t vk_command_pool, ShareTexture color,
                                   ShareTexture depth) {

    LLGI::CommandListVulkan::UseExternalCommandBuffer = true;

    auto vkQueueHandle = (VkQueue) vk_queue;
    auto vkDeviceHandle = (VkDevice) vk_device;
    auto vkPhyDeviceHandle = (VkPhysicalDevice) vk_phy_device;
    //   auto vkCommandPoolHandle = (VkCommandPool) vk_command_pool;

    auto vkQueue = vk::Queue(vkQueueHandle);
    auto vkDevice = vk::Device(vkDeviceHandle);
    auto vkPhyDevice = vk::PhysicalDevice(vkPhyDeviceHandle);
    // auto vkCommandPool = vk::CommandPool(vkCommandPoolHandle);

    vk::CommandPoolCreateInfo cmdPoolInfo;
    cmdPoolInfo.queueFamilyIndex = 0;
    cmdPoolInfo.flags = vk::CommandPoolCreateFlagBits::eResetCommandBuffer;
    auto vkCommandPool = vkDevice.createCommandPool(cmdPoolInfo);

    auto addCommand = [vkQueue](vk::CommandBuffer commandBuffer, vk::Fence fence) -> void {
//        std::array<vk::SubmitInfo, 1> copySubmitInfos;
//        copySubmitInfos[0].commandBufferCount = 1;
//        copySubmitInfos[0].pCommandBuffers = &commandBuffer;
//        vkQueue.submit(static_cast<uint32_t>(copySubmitInfos.size()), copySubmitInfos.data(),
//                       fence);
    };

    auto graphics = new LLGI::GraphicsVulkan(
            vkDevice,
            vkQueue,
            vkCommandPool,
            vkPhyDevice,
            3,
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


    ::EffekseerRendererVulkan::RenderPassInformation renderPassInfo;
    renderPassInfo.DoesPresentToScreen = false;
    renderPassInfo.RenderTextureCount = 1;
    renderPassInfo.RenderTextureFormats[0] = static_cast<VkFormat>(color.format);
    renderPassInfo.DepthFormat = static_cast<VkFormat>(depth.format);

    Startup(graphics, 1, renderPassInfo);

    auto renderPass = graphics->CreateRenderPass(colorTexture, nullptr, depthTexture, nullptr);

    LLGI::Color8 colorClear;
    colorClear.R = 0;
    colorClear.G = 0;
    colorClear.B = 0;
    colorClear.A = 255;

    renderPass->SetClearColor(colorClear);
    renderPass->SetIsColorCleared(false);
    renderPass->SetIsDepthCleared(false);

    context->renderPass = renderPass;
    return 0;
}

int32_t TestCall(int32_t input) {
    return input + 1;
}