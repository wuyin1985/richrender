//
// Created by yin.wu on 2021/11/20.
//

#ifndef RICHEFFEKSEER_EXPORT_HPP
#define RICHEFFEKSEER_EXPORT_HPP

#include <stdint.h>

typedef void *CEffekseerRenderer;
typedef void *CEffekseerManager;

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint64_t image;
    uint64_t view;
    int32_t format;
    int32_t width;
    int32_t height;
} ShareTexture;

__declspec( dllexport ) void RunWithPlatform();


__declspec( dllexport ) void Startup(void *graphic, void *renderPass);

__declspec( dllexport ) void UpdateFrame(void *renderPass);

__declspec( dllexport ) void Shutdown();

__declspec( dllexport ) int StartupWithExternalVulkan(uint64_t vk_device, uint64_t vk_phy_device,
                                                      uint64_t vk_queue, uint64_t vk_command_pool,
                                                      ShareTexture color, ShareTexture depth);

__declspec( dllexport ) int32_t TestCall(int32_t input);

#ifdef __cplusplus
}
#endif

#endif //RICHEFFEKSEER_EXPORT_HPP
