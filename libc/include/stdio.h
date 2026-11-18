#pragma once
#include <stddef.h>
#include <stdarg.h>
typedef struct KFILE FILE;
extern FILE *stdin, *stdout, *stderr;
#define EOF (-1)
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2
FILE *fopen(const char *path, const char *mode);
int fclose(FILE *f);
size_t fread(void *p, size_t size, size_t n, FILE *f);
size_t fwrite(const void *p, size_t size, size_t n, FILE *f);
int fseek(FILE *f, long off, int whence);
long ftell(FILE *f);
int fflush(FILE *f);
int fputs(const char *s, FILE *f);
int puts(const char *s);
int putchar(int c);
int printf(const char *fmt, ...);
int fprintf(FILE *f, const char *fmt, ...);
int sprintf(char *buf, const char *fmt, ...);
int snprintf(char *buf, size_t n, const char *fmt, ...);
int vsnprintf(char *buf, size_t n, const char *fmt, va_list ap);
int vfprintf(FILE *f, const char *fmt, va_list ap);
int sscanf(const char *s, const char *fmt, ...);
int remove(const char *path);
int rename(const char *a, const char *b);