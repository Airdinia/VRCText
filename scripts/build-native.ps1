# SPDX-License-Identifier: MIT
param(
    [Parameter(Mandatory=$true)][string]$SherpaSource,
    [Parameter(Mandatory=$true)][string]$OnnxSource,
    [string]$BuildDirectory = (Join-Path $PSScriptRoot '../target/native-build')
)
$ErrorActionPreference = 'Stop'
$taskSherpa = (Resolve-Path -LiteralPath $SherpaSource).Path
$taskOnnx = (Resolve-Path -LiteralPath $OnnxSource).Path
$taskRecipe = Join-Path $PSScriptRoot '../third-party/onnx-static'
$taskBuild = [IO.Path]::GetFullPath($BuildDirectory)
$taskProtoc = (Get-Command protoc.exe -ErrorAction Stop).Source
New-Item -ItemType Directory -Force -Path $taskBuild | Out-Null
function Invoke-CMake([string[]]$Arguments) {
    & cmake @Arguments
    if ($LASTEXITCODE -ne 0) { throw "CMake failed: $LASTEXITCODE" }
}

# Apply the same source adjustment as upstream build-static_lib.sh.
$taskOnnxCmake = Join-Path $taskOnnx 'cmake/onnxruntime.cmake'
$taskText = [IO.File]::ReadAllText($taskOnnxCmake)
$taskText = [regex]::Replace($taskText, '(?m)^.*SOVERSION.*\r?\n', '')
[IO.File]::WriteAllText($taskOnnxCmake, $taskText, [Text.UTF8Encoding]::new($false))
$taskOrtBuild = Join-Path $taskBuild 'onnxruntime'
$taskOrtInstall = Join-Path $taskBuild 'onnxruntime-install'
Invoke-CMake @('-S', $taskRecipe, '-B', $taskOrtBuild,
    '-G', 'Visual Studio 17 2022', '-A', 'x64',
    '-DCMAKE_BUILD_TYPE=Release', '-DCMAKE_CONFIGURATION_TYPES=Release',
    '-Dgtest_force_shared_crt=OFF',
    '-Donnxruntime_BUILD_UNIT_TESTS=OFF',
    "-DONNXRUNTIME_SOURCE_DIR=$taskOnnx", "-DONNX_CUSTOM_PROTOC_EXECUTABLE=$taskProtoc",
    "-DCMAKE_INSTALL_PREFIX=$taskOrtInstall", '--compile-no-warning-as-error')
Invoke-CMake @('--build', $taskOrtBuild, '--config', 'Release', '--parallel')
Invoke-CMake @('--install', $taskOrtBuild, '--config', 'Release')

$taskSherpaBuild = Join-Path $taskBuild 'sherpa'
$taskSherpaInstall = Join-Path $taskBuild 'sherpa-install'
Invoke-CMake @('-S', $taskSherpa, '-B', $taskSherpaBuild,
    '-G', 'Visual Studio 17 2022', '-A', 'x64',
    '-DCMAKE_BUILD_TYPE=Release', '-DSHERPA_ONNX_ENABLE_TTS=ON',
    '-DSHERPA_ONNX_USE_STATIC_CRT=ON', '-DSHERPA_ONNX_ENABLE_PORTAUDIO=OFF',
    '-DBUILD_SHARED_LIBS=OFF', '-DBUILD_ESPEAK_NG_EXE=OFF',
    "-DFETCHCONTENT_SOURCE_DIR_ONNXRUNTIME=$taskOrtInstall",
    "-DCMAKE_INSTALL_PREFIX=$taskSherpaInstall")
Invoke-CMake @('--build', $taskSherpaBuild, '--config', 'Release', '--parallel')
Invoke-CMake @('--install', $taskSherpaBuild, '--config', 'Release')
Write-Output "Use this directory as SHERPA_ONNX_LIB_DIR: $(Join-Path $taskSherpaInstall 'lib')"
