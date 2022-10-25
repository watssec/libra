char g_char[10] = "abcdefg";
int g_int[2] = {1, 2};

struct opaque;
struct opaque *g_opaque[4] = {0, 0, 0};

struct simple {
  char a;
  long b;
  void *c;
};
struct simple g_simple[14] = {{'a', 1, 0}, {'b', 2, 0}};