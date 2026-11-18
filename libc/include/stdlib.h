#pragma once
#include <stddef.h>
#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
void *malloc(size_t n);
void *calloc(size_t n, size_t size);
void *realloc(void *p, size_t n);
void free(void *p);
int atoi(const char *s);
double atof(const char *s);
int abs(int x);
long labs(long x);
int rand(void);
void srand(unsigned seed);
char *getenv(const char *name);
int system(const char *cmd);
void qsort(void *base, size_t n, size_t size, int (*cmp)(const void *, const void *));
__attribute__((noreturn)) void exit(int code);
__attribute__((noreturn)) void abort(void);