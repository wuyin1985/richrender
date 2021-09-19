glslc.exe simple_draw_object_vert.glsl -fshader-stage=vertex -o simple_draw_object_vert.spv -DIN_NORMAL
glslc.exe simple_draw_object_frag.glsl -fshader-stage=fragment -o simple_draw_object_frag.spv -DIN_NORMAL 
pause
