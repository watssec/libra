#ifndef MOCK_STDARG_H
#define MOCK_STDARG_H

typedef __builtin_va_list va_list;
#define va_start(ap, param) __builtin_va_start(ap, param)
#define va_end(ap) __builtin_va_end(ap)
#define va_arg(ap, type) __builtin_va_arg(ap, type)
#define va_copy(ap, s) __builtin_va_copy(ap, s)

#endif // MOCK_STDARG_H
