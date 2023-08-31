cmake_minimum_required(VERSION 3.22.1)
project(libra)

# options
set(CFG_LLVM_INSTALL_DIR "" CACHE PATH "LLVM installation directory")

# load LLVM
find_package(LLVM REQUIRED
        PATHS "${CFG_LLVM_INSTALL_DIR}/lib/cmake/llvm/"
        NO_DEFAULT_PATH)

# headers
include_directories(SYSTEM ${LLVM_INCLUDE_DIRS})
include_directories(${CMAKE_SOURCE_DIR})

# flags
set(CMAKE_CXX_STANDARD 17 CACHE STRING "")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra")
if (NOT LLVM_ENABLE_RTTI)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-rtti")
endif ()

# helpers
function(add_llvm_pass name)
    add_library(${name} SHARED ${ARGN})
endfunction()

# targets
add_subdirectory(Libra)
