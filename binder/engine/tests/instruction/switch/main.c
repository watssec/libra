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

void baz(char v) {
  int a;
  switch (v) {
  default:
    a = 0;
  case 0:
  case 1:
  case 2:
    a = 1;
    break;
  }
}