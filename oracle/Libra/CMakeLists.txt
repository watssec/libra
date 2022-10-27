# target
add_llvm_pass(Libra
              Config.hpp
              Logger.cpp
              Metadata.cpp
              SerializeAsm.cpp
              SerializeConstant.cpp
              SerializeFunction.cpp
              SerializeGlobalVariable.cpp
              SerializeInstruction.cpp
              SerializeModule.cpp
              SerializeType.cpp
              SerializeValue.cpp
              SerializerContext.cpp
              Pass.cpp)
