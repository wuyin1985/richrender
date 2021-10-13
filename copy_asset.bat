rd /s/q "target\debug\assets"
rd /s/q "target\release\assets"
del /s/q "assets\spv\temp\*.spv"
xcopy assets target\debug\assets\ /s /e
xcopy assets target\release\assets\ /s /e
