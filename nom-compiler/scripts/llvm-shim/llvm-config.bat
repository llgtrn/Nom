@echo off
setlocal enabledelayedexpansion
set "arg=%~1"
if "%arg%"=="--version" (echo 18.1.8)
if "%arg%"=="--prefix" (echo C:/Program Files/LLVM)
if "%arg%"=="--libdir" (echo C:/Program Files/LLVM/lib)
if "%arg%"=="--includedir" (echo C:/Program Files/LLVM/include)
if "%arg%"=="--cflags" (echo -IC:/Program Files/LLVM/include)
if "%arg%"=="--cxxflags" (echo -IC:/Program Files/LLVM/include)
if "%arg%"=="--ldflags" (echo -LC:/Program Files/LLVM/lib)
if "%arg%"=="--system-libs" (echo.)
if "%arg%"=="--libs" (echo -lLLVM-C)
if "%arg%"=="--shared-mode" (echo shared)
if "%arg%"=="--link-shared" (echo.)
if "%arg%"=="--link-static" (echo.)
if "%arg%"=="--has-rtti" (echo YES)
if "%arg%"=="--assertion-mode" (echo OFF)
if "%arg%"=="--components" (echo all)
if "%arg%"=="--targets-built" (echo AArch64 AMDGPU ARM AVR BPF Hexagon Lanai LoongArch Mips MSP430 NVPTX PowerPC RISCV Sparc SystemZ VE WebAssembly X86 XCore)
if "%arg%"=="--host-target" (echo x86_64-pc-windows-msvc)
if "%arg%"=="--build-mode" (echo Release)
endlocal
