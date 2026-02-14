cmake_minimum_required(VERSION 3.31)
project(dictype)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -Werror")
set(CMAKE_CXX_COMPILER /usr/bin/clang++)

set(CMAKE_CXX_CLANG_TIDY
        clang-tidy;
        --config-file=${CMAKE_SOURCE_DIR}/.clang-tidy;
        -warnings-as-errors=*;
)

find_program(CLANG_FORMAT_EXE NAMES clang-format)

# ---------- testing ----------
if (BUILD_TESTING)
    include(CTest)
    find_package(GTest REQUIRED)
endif ()

# ---------- gRPC / Protobuf code generation ----------
find_package(gRPC CONFIG REQUIRED)

set(DICTYPE_PROTO "${CMAKE_SOURCE_DIR}/proto/dictype.proto")
set(GENERATED_DIR "${CMAKE_BINARY_DIR}/__generated__/grpc")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

set(DICTYPE_PB "${GENERATED_DIR}/dictype.pb.cc")
set(DICTYPE_PB_H "${GENERATED_DIR}/dictype.pb.h")
set(DICTYPE_GRPC_PB "${GENERATED_DIR}/dictype.grpc.pb.cc")
set(DICTYPE_GRPC_PB_H "${GENERATED_DIR}/dictype.grpc.pb.h")

add_custom_command(
        OUTPUT ${DICTYPE_PB} ${DICTYPE_PB_H} ${DICTYPE_GRPC_PB} ${DICTYPE_GRPC_PB_H}
        COMMAND $<TARGET_FILE:protobuf::protoc>
        --proto_path=${CMAKE_SOURCE_DIR}/proto
        --cpp_out=${GENERATED_DIR}
        --grpc_out=${GENERATED_DIR}
        --plugin=protoc-gen-grpc=$<TARGET_FILE:gRPC::grpc_cpp_plugin>
        ${DICTYPE_PROTO}
        DEPENDS ${DICTYPE_PROTO}
        CODEGEN
        COMMENT "Generating C++ gRPC client"
        VERBATIM
)

# A phony target representing code generation
add_custom_target(dictype_grpc_codegen
        DEPENDS
        "${DICTYPE_GRPC_PB}"
        "${DICTYPE_GRPC_PB_H}"
)

add_library(dictype_grpc_client STATIC
        "${DICTYPE_PB}"
        "${DICTYPE_GRPC_PB}"
)
add_dependencies(dictype_grpc_client dictype_grpc_codegen)
# Disable clang-tidy on generated code and build with PIC for shared linking
set_target_properties(dictype_grpc_client PROPERTIES
        CXX_CLANG_TIDY ""
        POSITION_INDEPENDENT_CODE ON
)
# Include directory for generated headers
target_include_directories(dictype_grpc_client PUBLIC "${GENERATED_DIR}")
# Link libraries
target_link_libraries(dictype_grpc_client PUBLIC gRPC::grpc++ protobuf::libprotobuf)

# ---------- format check ----------
if (CLANG_FORMAT_EXE)
    file(GLOB_RECURSE ALL_CXX
            CONFIGURE_DEPENDS
            "${CMAKE_SOURCE_DIR}/DictypeFcitx/*.[ch]pp"
            "${CMAKE_SOURCE_DIR}/DictypeFcitx/*.h"
    )

    add_custom_target(format
            COMMAND ${CLANG_FORMAT_EXE} --color -i --Werror --style=file ${ALL_CXX}
            COMMENT "Running clang-format"
    )

    add_custom_target(format-check
            COMMAND ${CLANG_FORMAT_EXE} --color --dry-run --Werror --style=file ${ALL_CXX}
            COMMENT "Checking clang-format compliance"
    )
endif ()

add_subdirectory(DictypeFcitx)
