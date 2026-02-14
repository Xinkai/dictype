find_package(Fcitx5Core REQUIRED)

include("${FCITX_INSTALL_CMAKECONFIG_DIR}/Fcitx5Utils/Fcitx5Macros.cmake")
include("${FCITX_INSTALL_CMAKECONFIG_DIR}/Fcitx5Utils/Fcitx5CompilerSettings.cmake")
include("${FCITX_INSTALL_CMAKECONFIG_DIR}/Fcitx5Utils/Fcitx5UtilsConfigVersion.cmake")

set(DICTYPE_FCITX_VERSION 0.0.2)

add_library(dictype MODULE
        src/DictypeFcitx.cpp
        src/DictypeFcitx.h
        src/DictypeState.cpp
        src/DictypeState.h
        src/DictypeConfig.h
        src/GrpcClient.cpp
        src/GrpcClient.h
)
target_link_libraries(dictype PRIVATE
        Fcitx5::Core
        Fcitx5::Config
        dictype_grpc_client
)
add_dependencies(dictype dictype_grpc_codegen)
install(TARGETS dictype DESTINATION "${FCITX_INSTALL_ADDONDIR}")

configure_file(dictype.conf.in dictype.conf @ONLY)
install(
        FILES "${CMAKE_CURRENT_BINARY_DIR}/dictype.conf"
        DESTINATION "${FCITX_INSTALL_PKGDATADIR}/addon"
        COMPONENT config
)

if (BUILD_TESTING)
    include(GoogleTest)
    add_executable(dictype_state_tests
            src/DictypeState.cpp
            src/DictypeState.test.cpp
    )
    target_link_libraries(dictype_state_tests PRIVATE
            GTest::gtest_main
            Fcitx5::Core
            Fcitx5::Config
            dictype_grpc_client
    )
    gtest_discover_tests(dictype_state_tests)
endif ()
