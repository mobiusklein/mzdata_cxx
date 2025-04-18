cmake_minimum_required(VERSION 3.22)
project(mzdata_cxx)

set(CMAKE_CXX_STANDARD 20)

set(CARGO_MANIFEST ${CMAKE_SOURCE_DIR}/Cargo.toml)
set(CARGO_TARGET_DIR ${CMAKE_SOURCE_DIR}/target)

set(MZDATA_SOURCE_FILE ${CMAKE_SOURCE_DIR}/src/lib.rs)
set(MZDATA_BRIDGE_CPP ${CARGO_TARGET_DIR}/cxxbridge/mzdata_cxx/src/lib.rs.cc)
set(MZDATA_LIB ${CARGO_TARGET_DIR}/release/${CMAKE_STATIC_LIBRARY_PREFIX}mzdata_cxx${CMAKE_STATIC_LIBRARY_SUFFIX})

# Add a custom command that builds the rust crate and generates C++ bridge code
add_custom_command(
        OUTPUT ${MZDATA_BRIDGE_CPP} ${MZDATA_LIB}
        COMMAND cargo build -r --manifest-path ${CARGO_MANIFEST}
        DEPENDS ${MZDATA_SOURCE_FILE}
        USES_TERMINAL
        COMMENT "Running cargo..."
)

add_executable(mzdata_cxx_test test/test.cpp ${MZDATA_BRIDGE_CPP})

target_include_directories(
        mzdata_cxx_test
        PRIVATE
        include/
        ${CARGO_TARGET_DIR}/cxxbridge/mzdata_cxx/src/
        ${CARGO_TARGET_DIR}/cxxbridge/rust/
)


target_link_libraries(mzdata_cxx_test ${MZDATA_LIB})

set_target_properties(
        mzdata_cxx_test
        PROPERTIES ADDITIONAL_CLEAN_FILES ${CARGO_TARGET_DIR}
)


# Windows-only configuration
if(WIN32)
    target_link_libraries(mzdata_cxx_test userenv kernel32 ntdll ws2_32 bcrypt)
    set_target_properties(
            mzdata_cxx_test
            PROPERTIES
            MSVC_RUNTIME_LIBRARY "MultiThreadedDLL"
            RUNTIME_OUTPUT_DIRECTORY_DEBUG ${CMAKE_BINARY_DIR}
            RUNTIME_OUTPUT_DIRECTORY_RELEASE ${CMAKE_BINARY_DIR}
    )
endif()
