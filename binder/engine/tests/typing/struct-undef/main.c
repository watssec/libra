struct undefined;

struct defined_named {
  struct undefined *f;
};
struct defined_named g_named;

struct {
  struct undefined *f;
} g_anon;