#ifndef LIBRA_SERIALIZER_H
#define LIBRA_SERIALIZER_H

#include "Deps.h"

namespace libra {

json::Object serialize_module(const Module &module);

}

#endif // LIBRA_SERIALIZER_H
