void foo(long v) {
  int a;
  switch (v) {
  case 0:
    a = 1;
    break;
  case 1:
    a = 2;
    break;
  case 2:
    a = 3;
    break;
  default:
    a = 4;
  }
}

void bar(char v) {
  int a;
  switch (v) {
  case 0:
    a = 1;
    break;
  case 1:
    a = 2;
    break;
  case 2:
    a = 3;
    break;
  }
}