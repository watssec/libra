# target
add_llvm_pass(Libra
              Config.hpp
              Logger.cpp
              MetadataFunction.cpp
              SerializeConstant.cpp
              SerializeFunction.cpp
              SerializeGlobalVariable.cpp
              SerializeInstruction.cpp
              SerializeModule.cpp
              SerializeType.cpp
              Pass.cpp)
