cmake_minimum_required(VERSION 3.7)
# Solution name
project(RichEffekseerTest C CXX)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)

set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)

set(CMAKE_SIZEOF_VOID_P 8)

set(RichEffekseerLibName RichEffekseer)

set(BUILD_VULKAN ON)
set(BUILD_DX9 OFF)
set(BUILD_DX11 OFF)
set(BUILD_DX12 OFF)
set(BUILD_GL OFF)
set(USE_OPENAL OFF)
set(USE_XAUDIO2 OFF)
set(USE_DSOUND OFF)
set(USE_OSM OFF)
set(BUILD_EXAMPLES OFF)
set(USE_MSVC_RUNTIME_LIBRARY_DLL ON)

set(EFK_RUNTIME_ROOT Effekseer/Dev/Cpp)

find_package(Vulkan REQUIRED)

add_subdirectory(Effekseer)

add_library(${RichEffekseerLibName} SHARED export.cpp)

message(STATUS "root : ${EFK_RUNTIME_ROOT}")

target_include_directories(
        ${RichEffekseerLibName}
        PRIVATE
        ${Vulkan_INCLUDE_DIRS}
        ${EFK_RUNTIME_ROOT}/Effekseer
        ${EFK_RUNTIME_ROOT}/EffekseerRendererVulkan
        ${EFK_THIRDPARTY_INCLUDES}
        ${EFK_RUNTIME_ROOT}/3rdParty/LLGI/src/
        include
)

target_link_libraries(
        ${RichEffekseerLibName}
        PRIVATE
        Effekseer
        LLGI
        EffekseerRendererVulkan
        ${Vulkan_LIBRARIES}
        glslang
        SPIRV
        OGLCompiler
        OSDependent
)

include_directories(include)

install(TARGETS ${RichEffekseerLibName}
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
        PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})


add_executable(${PROJECT_NAME} main.c)

target_link_libraries(
        ${PROJECT_NAME}
        PRIVATE
        RichEffekseer
)

include_directories(
        ${PROJECT_NAME}
        RichEffekseer/include
)

set_target_properties(${PROJECT_NAME} PROPERTIES LINKER_LANGUAGE C)

add_custom_command(TARGET RichEffekseer POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy ${CMAKE_BINARY_DIR}/bin/RichEffekseer.dll ../../../lib/RichEffekseer.dll
        COMMAND ${CMAKE_COMMAND} -E copy ${CMAKE_BINARY_DIR}/bin/RichEffekseer.pdb ../../../lib/RichEffekseer.pdb
        COMMAND ${CMAKE_COMMAND} -E copy ${CMAKE_BINARY_DIR}/bin/RichEffekseer.dll ../../../target/debug/RichEffekseer.dll
        COMMAND ${CMAKE_COMMAND} -E copy ${CMAKE_BINARY_DIR}/bin/RichEffekseer.pdb ../../../target/debug/RichEffekseer.pdb
        COMMAND ${CMAKE_COMMAND} -E copy ${CMAKE_BINARY_DIR}/bin/RichEffekseer.dll ../../../target/release/RichEffekseer.dll
        COMMAND ${CMAKE_COMMAND} -E copy ${CMAKE_BINARY_DIR}/bin/RichEffekseer.pdb ../../../target/release/RichEffekseer.pdb
        COMMAND ${CMAKE_COMMAND} -E copy ${CMAKE_BINARY_DIR}/RichEffekseer.lib ../../../lib/RichEffekseer.lib
        )
