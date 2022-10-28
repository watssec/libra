void test(void) {
  int i_s = 2;
  unsigned int i_u = 3;

  int i_s_add = i_s + i_s + 1;
  int i_s_sub = i_s - i_s - 2;
  int i_s_mul = i_s * i_s * 3;
  int i_s_div = i_s / i_s / 4;
  int i_s_mod = i_s % i_s % 5;
  int i_s_shl = i_s << i_s << 6;
  int i_s_shr = i_s >> i_s >> 7;
  int i_s_and = i_s & i_s & 8;
  int i_s_or = i_s | i_s | 9;
  int i_s_xor = i_s ^ i_s ^ 10;

  unsigned int i_u_add = i_u + i_u + 1;
  unsigned int i_u_sub = i_u - i_u - 2;
  unsigned int i_u_mul = i_u * i_u * 3;
  unsigned int i_u_div = i_u / i_u / 4;
  unsigned int i_u_mod = i_u % i_u % 5;
  unsigned int i_u_shl = i_u << i_u << 6;
  unsigned int i_u_shr = i_u >> i_u >> 7;
  unsigned int i_u_and = i_u & i_u & 8;
  unsigned int i_u_or = i_u | i_u | 9;
  unsigned int i_u_xor = i_u ^ i_u ^ 10;
}