void *g_void = 0;
int *g_int = 0;
void (*g_fun_ptr)(int) = 0;

struct opaque;
struct opaque *g_opaque = 0;

struct defined {
  int a;
  void *b;
  struct opaque *c;
  struct defined *d;
  void (*e)(int);
};
struct defined *g_defined = 0;