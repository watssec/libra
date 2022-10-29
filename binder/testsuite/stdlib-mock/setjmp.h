#ifndef MOCK_SETJMP_H
#define MOCK_SETJMP_H

typedef long int __jmp_buf[8];

typedef struct __jmp_buf_tag {
  __jmp_buf __jb;
  unsigned long __fl;
  unsigned long __ss[128 / sizeof(long)];
} jmp_buf[1];

int setjmp(jmp_buf env) { return 0; }
void longjmp(jmp_buf env, int val) {}

typedef jmp_buf sigjmp_buf;
int sigsetjmp(sigjmp_buf env, int val) { return 0; }
void siglongjmp(sigjmp_buf env, int val) {}

#endif // MOCK_SETJMP_H
