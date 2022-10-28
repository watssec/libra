struct simple {
  int f_int;
  void *f_ptr_void;
};

struct complex {
  long f_arr_long[16];
  struct simple *f_ptr_simple;
  struct simple f_arr_simple[32];
};

void foo(struct complex *base, long index) {
  long t1 = base->f_arr_long[index];
  int t2 = base->f_ptr_simple->f_int;
  void *t3 = base->f_arr_simple[12].f_ptr_void;
  int *t4 = &base[2].f_ptr_simple[6].f_int;
}
