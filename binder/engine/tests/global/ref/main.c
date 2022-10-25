int g_int;
int *g_ptr_int = &g_int;
int *g_ptr_int2 = &g_int;

long g_long;
void *g_ptr_void[2] = {&g_int, &g_long};

void **g_ptr_ptr_void = g_ptr_void;
void *g_ptr_ptr_void_as_ptr = g_ptr_void;

struct simple {
  int *f_int;
  long *g_long;
  void **g_ptr_void;
};
struct simple g_simple = {&g_int, &g_long, g_ptr_void};

void func_1(int a) {}
void (*g_ptr_func_1)(int) = func_1;