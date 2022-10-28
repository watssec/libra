struct simple {
  int f_int;
  long f_long;
};

void test(void) {
  int i_s = 2;
  unsigned int i_u = i_s;
  long l_s = i_u;
  _Bool b_u = l_s;

  void *p_void = &i_s;
  int *p_int = p_void;
  long *p_long = p_void;
  void *p_void_2 = p_long;
  struct simple *p_simple = p_void_2;

  long l_s_from_ptr = (long)p_simple;
  unsigned long l_u_from_ptr = (unsigned long)p_void_2;
  int *p_int_from_l_s = (int *)l_s_from_ptr;
  _Bool *p_bool_from_l_u = (_Bool *)p_void_2;
}