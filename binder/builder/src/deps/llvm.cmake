# General
set(CMAKE_BUILD_TYPE Release CACHE STRING "")

# Stage 1 setup
set(CLANG_ENABLE_BOOTSTRAP ON CACHE BOOL "")
set(CLANG_BOOTSTRAP_TARGETS
    clang
    check-all
    check-llvm
    check-clang
    test-suite
    stage3
    stage3-clang
    stage3-check-all
    stage3-check-llvm
    stage3-check-clang
    stage3-test-suite
    stage3-install CACHE STRING "")

# Stage 1: build core with system cc
#          the new clang will have -flto=<?> support
set(STAGE1_PROJECTS "clang;lld")
set(STAGE1_RUNTIMES "")

set(LLVM_TARGETS_TO_BUILD Native CACHE STRING "")
set(LLVM_ENABLE_PROJECTS ${STAGE1_PROJECTS} CACHE STRING "")
set(LLVM_ENABLE_RUNTIMES ${STAGE1_RUNTIMES} CACHE STRING "")

# Stage 2 setup
set(BOOTSTRAP_CLANG_ENABLE_BOOTSTRAP ON CACHE STRING "")
set(BOOTSTRAP_CLANG_BOOTSTRAP_TARGETS
    clang
    check-all
    check-llvm
    check-clang CACHE STRING "")

# Stage 2: build core with stage1-clang -flto=thin
#          the new clang will have -flto=<?> support
set(STAGE2_PROJECTS "clang;lld")
set(STAGE2_RUNTIMES "")

set(BOOTSTRAP_LLVM_TARGETS_TO_BUILD Native CACHE STRING "")
set(BOOTSTRAP_LLVM_ENABLE_PROJECTS ${STAGE2_PROJECTS} CACHE STRING "")
set(BOOTSTRAP_LLVM_ENABLE_RUNTIMES ${STAGE2_RUNTIMES} CACHE STRING "")
set(BOOTSTRAP_LLVM_ENABLE_LLD ON CACHE BOOL "")
set(BOOTSTRAP_LLVM_ENABLE_LTO "Thin" CACHE STRING "")

# Stage 3: build llvm with stage2-clang -flto=full
#          the new clang will have -flto=<?> and -stdlib=libc++ support
set(STAGE3_PROJECTS "clang;clang-tools-extra;lld;lldb;polly;libc")
set(STAGE3_RUNTIMES "compiler-rt;libcxx;libcxxabi;libunwind")

set(BOOTSTRAP_BOOTSTRAP_LLVM_TARGETS_TO_BUILD Native CACHE STRING "")
set(BOOTSTRAP_BOOTSTRAP_LLVM_ENABLE_PROJECTS ${STAGE3_PROJECTS} CACHE STRING "")
set(BOOTSTRAP_BOOTSTRAP_LLVM_ENABLE_RUNTIMES ${STAGE3_RUNTIMES} CACHE STRING "")
set(BOOTSTRAP_BOOTSTRAP_LLVM_ENABLE_LLD ON CACHE BOOL "")
set(BOOTSTRAP_BOOTSTRAP_LLVM_ENABLE_LTO "Full" CACHE STRING "")
set(BOOTSTRAP_BOOTSTRAP_LLVM_ENABLE_RTTI ON CACHE BOOL "")
