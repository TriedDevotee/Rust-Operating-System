#pragma once
#include <stddef.h>
#include <sys/types.h>
#define F_OK 0
#define W_OK 2
#define R_OK 4
int close(int fd);
ssize_t read(int fd, void *buf, size_t n);
ssize_t write(int fd, const void *buf, size_t n);
off_t lseek(int fd, off_t off, int whence);
int access(const char *path, int mode);
int unlink(const char *path);
char *getcwd(char *buf, size_t n);