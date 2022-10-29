#ifndef MOCK_STDLIB_H
#define MOCK_STDLIB_H

#include <stdint.h>

void *malloc(size_t size) { return 0; }
void free(void *ptr) {}

#endif // MOCK_STDLIB_H
