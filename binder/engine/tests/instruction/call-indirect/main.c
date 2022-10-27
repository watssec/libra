void foo(int a) {}

void (*g_fun_ptr_null)(int) = 0;
void (*g_fun_ptr_foo)(int) = foo;

void bar(int a) {
  g_fun_ptr_null(0);
  foo(a);
}
