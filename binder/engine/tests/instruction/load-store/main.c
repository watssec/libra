long g;

long foo(long v) {
  long *ptr_v = &v;
  long a = *ptr_v;
  *ptr_v = 0;

  long *ptr_a = &a;
  *ptr_a = *ptr_v;

  g = a;
  long *ptr_g = &g;
  *ptr_g = *ptr_a;

  return *ptr_g;
}
