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

typedef struct {
    float Values[4][4];
} Matrix;

__declspec( dllexport ) void RunWithPlatform();

__declspec( dllexport ) void UpdateFrame(void *renderPass, uint64_t externalCommandBufferHandle);

__declspec( dllexport ) void Shutdown();

__declspec( dllexport ) uint64_t
StartupWithExternalVulkan(uint64_t vk_device, uint64_t vk_phy_device,
                          uint64_t vk_queue, uint64_t
                          vk_command_pool, ShareTexture color, ShareTexture depth);


__declspec( dllexport ) int32_t LoadEffectPrefab(const void *effectData, int size, void *
path);

__declspec( dllexport ) void ReleaseEffectPrefab(int32_t handle);

__declspec( dllexport ) int32_t PlayEffect(int32_t handle);

__declspec( dllexport ) void StopEffect(int32_t handle);

__declspec( dllexport ) void SetEffectLocation(int32_t handle, float x, float y, float z);

__declspec( dllexport ) void SetEffectRotation(int32_t handle, float x, float y, float z);

__declspec( dllexport ) void SyncProjectionMatrix(Matrix matrix);

__declspec( dllexport ) void SyncViewMatrix(Matrix matrix);

__declspec(dllexport) void SetThreadLockCall(void (*lock)(), void (*unlock)());

#ifdef __cplusplus
}
#endif

#endif //RICHEFFEKSEER_EXPORT_HPP
