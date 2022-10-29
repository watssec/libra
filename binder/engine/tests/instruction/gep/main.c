struct simple {
  int f_int;
  void *f_ptr_void;
};

struct complex {
  long f_arr_long[16];
  struct simple *f_ptr_simple;
  struct simple f_arr_simple[32];
};

void foo(struct complex *base, int a_int, long a_long) {
  struct complex *p_ptr_base_offset_const = &base[2];
  struct complex *p_ptr_base_offset_int = &base[a_int];
  struct complex *p_ptr_base_offset_long = &base[a_long];

  long t1 = base->f_arr_long[2];
  int t2 = base->f_ptr_simple->f_int;
  void *t3 = base->f_arr_simple[a_int].f_ptr_void;
  int *t4 = &base[a_long].f_ptr_simple[a_int].f_int;
  int *t5 = &base->f_arr_simple[a_long].f_int;
  struct simple *t6 = &base->f_arr_simple[a_int];
  struct simple *t7 = &base->f_arr_simple[a_long];
}