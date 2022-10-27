void fun_1() {}

void fun_2() {
  fun_1();
}

int fun_3(int a) {
  return a;
}

int fun_4() {
  return fun_3(0);
}

int fun_5(int a) {
  return fun_3(a);
}

int g_int;
int fun_6() {
  return fun_3(g_int);
}