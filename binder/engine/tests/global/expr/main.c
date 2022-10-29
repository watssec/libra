struct opaque;
struct simple {
  int a;
  long b;
};
struct defined {
  int a;
  void *b;
  struct opaque *c;
  struct defined *d;
  void (*e)(int);
  struct simple f[10];
};

const int g_int_const = 1 + 2;
long g_long_var = g_int_const * 1024;

const struct defined g_defined_const = {2, 0, 0, 0, 0, {3, 4}};
const int *g_int_gep = &g_defined_const.a;