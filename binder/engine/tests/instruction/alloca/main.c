void foo() {
  int src = 1;
  int dst;
  __asm__("mov %1, %0\n\t"
          "add $1, %0"
          : "=r"(dst)
          : "r"(src));
}

void bar() {
  int a = 10, b = 5;
  int c = 0;             // overflow flag
  __asm__("addl %2,%3;"  // Do a + b (the result goes into b)
          "jno 0f;"      // Jump ahead if an overflow occurred
          "movl $1, %1;" // Copy 1 into c
          "0:"           // We're done.
          : "=r"(b), "=m"(c) // Output list
          : "r"(a), "0"(b)   // Input list
  );
}