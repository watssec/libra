struct {
  unsigned int bf_1: 12;
  unsigned int: 0;
  unsigned int bf_2: 12;
} t1 = {1, 2};
static int a1[(sizeof(t1) == 8) -1];