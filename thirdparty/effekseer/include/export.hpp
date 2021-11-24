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
    void *image;
    void *view;
    int32_t format;
    int32_t width;
    int32_t height;
} ShareTexture;

__declspec( dllexport ) void RunWithPlatform();

__declspec( dllexport ) void Startup(void *graphic, void *renderPass);

__declspec( dllexport ) void UpdateFrame(void *renderPass);

__declspec( dllexport ) void Shutdown();

__declspec( dllexport ) int StartupWithExternalVulkan(void *vk_device, void *vk_phy_device,
                                                      void *vk_queue, void *vk_command_pool,
                                                      ShareTexture color, ShareTexture depth);


#ifdef __cplusplus
}
#endif

#endif //RICHEFFEKSEER_EXPORT_HPP
