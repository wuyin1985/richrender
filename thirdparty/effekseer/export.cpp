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

VkCommandBuffer GetCommandList();

int GetSwapBufferCount();


#include <LLGI.Graphics.h>
#include <LLGI.Platform.h>
#include <Utils/LLGI.CommandListPool.h>
#include <Vulkan/LLGI.CommandListVulkan.h>
#include <Vulkan/LLGI.GraphicsVulkan.h>

struct ContextLLGI {
    LLGI::Graphics *graphics;
    LLGI::RenderPass *renderPass;
    std::shared_ptr<LLGI::SingleFrameMemoryPool> memoryPool;
    std::shared_ptr<LLGI::CommandListPool> commandListPool;
    LLGI::CommandList *commandList = nullptr;
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

VkCommandBuffer GetCommandList() {
    return static_cast<VkCommandBuffer>(static_cast<LLGI::CommandListVulkan *>(context->commandList)->GetCommandBuffer());
}

int GetSwapBufferCount() { return 3; }


void Startup(void *vGraphic, void *vRenderPass) {
    context = std::make_shared<ContextLLGI>();
    context->graphics = reinterpret_cast<LLGI::Graphics *>(vGraphic);
    context->time = 0;

    if (vRenderPass != nullptr) {
        context->renderPass = reinterpret_cast<LLGI::RenderPass *>(vRenderPass);
    }

    context->memoryPool = LLGI::CreateSharedPtr(
            context->graphics->CreateSingleFrameMemoryPool(1024 * 1024, 128));
    context->commandListPool = std::make_shared<LLGI::CommandListPool>(context->graphics,
                                                                       context->memoryPool.get(),
                                                                       3);

    ::EffekseerRendererVulkan::RenderPassInformation renderPassInfo;
    renderPassInfo.DoesPresentToScreen = true;
    renderPassInfo.RenderTextureCount = 1;
    renderPassInfo.RenderTextureFormats[0] = VK_FORMAT_B8G8R8A8_UNORM;
    renderPassInfo.DepthFormat = VK_FORMAT_D24_UNORM_S8_UINT;
    auto renderer = ::EffekseerRendererVulkan::Create(
            GetVkPhysicalDevice(), GetVkDevice(), GetVkQueue(), GetVkCommandPool(),
            GetSwapBufferCount(), renderPassInfo, 8000);
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
    auto effect = Effekseer::Effect::Create(manager, u"Assets/test.efk", 1.0f, nullptr);
    context->effect = effect;
}

void Shutdown() {
    context->manager.Reset();

    context->renderer.Reset();

    context->graphics->WaitFinish();

    context = nullptr;
}


void UpdateFrame(void *vRenderPass) {
    LLGI::RenderPass *renderPass = context->renderPass;
    if (vRenderPass != nullptr) {
        renderPass = reinterpret_cast<LLGI::RenderPass *>(vRenderPass);
    }

    context->memoryPool->NewFrame();

    context->commandList = context->commandListPool->Get();

    LLGI::Color8 color;
    color.R = 0;
    color.G = 0;
    color.B = 0;
    color.A = 255;

    context->commandList->Begin();

    renderPass->SetClearColor(color);
    renderPass->SetIsColorCleared(true);
    renderPass->SetIsDepthCleared(true);

    context->commandList->BeginRenderPass(renderPass);

    context->sfMemoryPoolEfk->NewFrame();

    EffekseerRendererVulkan::BeginCommandList(context->commandListEfk, GetCommandList());
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

    context->commandList->EndRenderPass();

    context->commandList->End();

    context->graphics->Execute(context->commandList);

    context->time = time + 1;
}

