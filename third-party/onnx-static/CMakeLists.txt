# SPDX-License-Identifier: MIT
# VRCText's independent adapter for the upstream ONNX Runtime CMake project.
cmake_minimum_required(VERSION 3.28)
cmake_policy(SET CMP0091 NEW)
if(CMAKE_VERSION VERSION_GREATER_EQUAL "4.0")
  set(CMAKE_POLICY_VERSION_MINIMUM 3.5)
endif()
project(vrctext_onnx_static LANGUAGES C CXX)
set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded CACHE STRING "" FORCE)
set(ONNX_USE_MSVC_STATIC_RUNTIME ON CACHE BOOL "" FORCE)
set(protobuf_MSVC_STATIC_RUNTIME ON CACHE BOOL "" FORCE)
set(ABSL_MSVC_STATIC_RUNTIME ON CACHE BOOL "" FORCE)
set(onnxruntime_BUILD_SHARED_LIB OFF CACHE BOOL "" FORCE)
set(onnxruntime_BUILD_UNIT_TESTS OFF CACHE BOOL "" FORCE)
add_subdirectory("${ONNXRUNTIME_SOURCE_DIR}/cmake" runtime EXCLUDE_FROM_ALL)

function(vrctext_visit target)
  if(NOT TARGET "${target}")
    return()
  endif()
  get_target_property(alias "${target}" ALIASED_TARGET)
  if(alias)
    set(target "${alias}")
  endif()
  get_property(seen GLOBAL PROPERTY vrctext_seen)
  if("${target}" IN_LIST seen)
    return()
  endif()
  set_property(GLOBAL APPEND PROPERTY vrctext_seen "${target}")
  get_target_property(kind "${target}" TYPE)
  if(kind STREQUAL "STATIC_LIBRARY")
    set_property(GLOBAL APPEND PROPERTY vrctext_archives "\"$<TARGET_FILE:${target}>\"")
    set_property(GLOBAL APPEND PROPERTY vrctext_targets "${target}")
  endif()
  foreach(property LINK_LIBRARIES INTERFACE_LINK_LIBRARIES)
    get_target_property(links "${target}" "${property}")
    if(links)
      foreach(link IN LISTS links)
        if(link MATCHES "^\\$<(LINK_ONLY|BUILD_INTERFACE|TARGET_NAME_IF_EXISTS):([^>]+)>$")
          set(link "${CMAKE_MATCH_2}")
        endif()
        vrctext_visit("${link}")
      endforeach()
    endif()
  endforeach()
endfunction()

foreach(root onnxruntime_session onnxruntime_optimizer onnxruntime_providers
    onnxruntime_util onnxruntime_framework onnxruntime_graph onnxruntime_mlas
    onnxruntime_common onnxruntime_flatbuffers onnx_test_data_proto onnx onnx_proto
    libprotobuf-lite re2 absl_base absl_throw_delegate absl_raw_hash_set absl_hash
    absl_city absl_low_level_hash cpuinfo)
  vrctext_visit("${root}")
endforeach()
get_property(archives GLOBAL PROPERTY vrctext_archives)
get_property(targets GLOBAL PROPERTY vrctext_targets)
list(JOIN archives "\n" response)
file(GENERATE OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/archives-$<CONFIG>.rsp" CONTENT "${response}\n")
set(bundle "${CMAKE_CURRENT_BINARY_DIR}/onnxruntime.lib")
add_custom_command(OUTPUT "${bundle}"
  COMMAND "${CMAKE_AR}" /NOLOGO "/OUT:${bundle}" "@${CMAKE_CURRENT_BINARY_DIR}/archives-$<CONFIG>.rsp"
  DEPENDS ${targets} VERBATIM)
add_custom_target(vrctext_bundle ALL DEPENDS "${bundle}")
install(FILES "${bundle}" DESTINATION lib)
install(DIRECTORY "${ONNXRUNTIME_SOURCE_DIR}/include/onnxruntime/core/session/"
  DESTINATION include FILES_MATCHING PATTERN "*.h")
install(FILES "${ONNXRUNTIME_SOURCE_DIR}/include/onnxruntime/core/providers/cpu/cpu_provider_factory.h"
  "${ONNXRUNTIME_SOURCE_DIR}/include/onnxruntime/core/framework/provider_options.h"
  DESTINATION include)
