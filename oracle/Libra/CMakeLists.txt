# target
add_llvm_pass(Libra
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
