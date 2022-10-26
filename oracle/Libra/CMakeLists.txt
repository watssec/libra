# target
add_llvm_pass(Libra
              Config.hpp
              Logger.cpp
              MetadataFunction.cpp
              SerializeConstant.cpp
              SerializeGlobalVariable.cpp
              SerializeFunction.cpp
              SerializeModule.cpp
              SerializeType.cpp
              Pass.cpp)
