cmake_minimum_required(VERSION 3.5.0)
project(alex_crypto)

# Argon2 is a shared, platform-neutral source set. HarmonyOS and the KMP iOS
# actual both build this same implementation; keep the algorithm out of either
# platform directory so the two copies cannot drift.
set(ALEX_ARGON2_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../../../../../../../shared/argon2)

add_library(alex_crypto SHARED
  napi_argon2.cpp
  ${ALEX_ARGON2_DIR}/argon2.c
  ${ALEX_ARGON2_DIR}/core.c
  ${ALEX_ARGON2_DIR}/encoding.c
  ${ALEX_ARGON2_DIR}/ref.c
  ${ALEX_ARGON2_DIR}/run.c
  ${ALEX_ARGON2_DIR}/thread.c
  ${ALEX_ARGON2_DIR}/blake2/blake2b.c
)

target_include_directories(alex_crypto PRIVATE ${ALEX_ARGON2_DIR})
target_compile_definitions(alex_crypto PRIVATE A2_VISCTL)
target_link_libraries(alex_crypto PUBLIC libace_napi.z.so)
