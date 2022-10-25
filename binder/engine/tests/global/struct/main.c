struct simple {
  char a;
  long b;
  void *c;
  struct opaque *d;
};
struct simple g_simple = {'a', 1, 0, 0};

struct complex {
  struct simple f_simple;
  struct simple *f_ptr_simple;
  struct simple f_arr_simple[3];
  struct complex *f_ptr_complex;
};
struct complex g_complex = {{'a', 1}, 0, {{'b', 2}, {'c', 3}}, 0};