struct opaque;

struct simple {
  int a;
  long b;
};
struct simple g_simple = {0};

typedef int (*t_somefunc)(int, int);

struct complex {
  int f_int;
  long f_arr_long[1];
  int *f_ptr_int;
  unsigned long *f_arr_ptr_unsigned_long[2];
  void *f_ptr_void;
  void *f_arr_ptr_void[3];
  struct opaque *f_ptr_opaque;
  struct opaque *f_arr_ptr_opaque[4];
  struct simple f_simple;
  struct simple f_arr_simple[5];
  struct simple *f_ptr_simple;
  struct simple *f_arr_ptr_simple[6];
  struct complex *f_ptr_complex;
  struct complex *f_arr_ptr_complex[7];
  void (*f_fun_ptr)(int);
  t_somefunc f_fun_ptr_some;
  t_somefunc f_arr_fun_ptr_some[8];
};
struct complex g_defined = {0};