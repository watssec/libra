# target
add_llvm_pass(Libra
              Oracle.cpp
              Pass.cpp)
target_link_libraries(Libra Shared)
