void *g_void = 0;
int *g_int = 0;

struct opaque;
struct opaque* g_opaque = 0;

struct defined {
  int a;
  void* b;
  struct opaque* c;
  struct defined* d;
};
struct defined* g_defined = 0;