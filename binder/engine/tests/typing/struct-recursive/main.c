struct t1;

struct t2 {
  struct t1 *f;
};

struct t1 {
  struct t2 f;
};

struct t1 g1;
struct t2 g2;