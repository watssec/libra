void foo(long v) {
  long alloca_int = v;
  long alloca_arr_int_const[16] = {0};
  long alloca_arr_int_var1[v];
  long alloca_arr_int_var2[alloca_int];
}