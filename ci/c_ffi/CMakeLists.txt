cmake_minimum_required(VERSION 3.10)
project(gtfsort_cxx_ffi_test)

enable_testing()

set(CRATE_ROOT ${CMAKE_SOURCE_DIR}/../../)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if (MSVC)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /W4 /WX")
else()
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -Wpedantic -Werror")
endif()

include_directories(${CRATE_ROOT}/include)

add_library(gtfsort SHARED IMPORTED)

set_target_properties(gtfsort PROPERTIES IMPORTED_LOCATION ${CRATE_ROOT}/target/release/libgtfsort.so)

add_executable(gtfsort_cxx_ffi_test test_ffi.cxx)

target_link_libraries(gtfsort_cxx_ffi_test gtfsort)

add_test(NAME test_gtfsort_cxx_ffi_test COMMAND gtfsort_cxx_ffi_test ${CMAKE_CURRENT_BINARY_DIR}/data.gff3 ${CMAKE_CURRENT_BINARY_DIR}/output-mmap.gff3 ${CMAKE_CURRENT_BINARY_DIR}/output-str.gff3)

set(DATA_URL "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_mouse/release_M35/gencode.vM35.chr_patch_hapl_scaff.basic.annotation.gff3.gz")

add_custom_target(build_rust COMMAND cargo build --release --manifest-path ${CRATE_ROOT}/Cargo.toml --features "mmap c_ffi")

file(DOWNLOAD ${DATA_URL} ${CMAKE_CURRENT_BINARY_DIR}/data.gff3.gz SHOW_PROGRESS EXPECTED_HASH SHA256=ddc8fc9420180bf0d53a91222bff6f0eebe9d9ccbe4f0dac820f672ab6ac7257)
add_custom_target(extract_data COMMAND gunzip -k -f ${CMAKE_CURRENT_BINARY_DIR}/data.gff3.gz)

add_custom_target(check COMMAND ${CMAKE_CTEST_COMMAND} DEPENDS build_rust extract_data gtfsort_cxx_ffi_test)