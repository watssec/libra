#ifndef MOCK_STDDEF_H
#define MOCK_STDDEF_H

#include <stdint.h>

#define NULL ((void *)0)

#define offsetof(TYPE, MEMBER) __builtin_offsetof(TYPE, MEMBER)

#endif // MOCK_STDDEF_H
