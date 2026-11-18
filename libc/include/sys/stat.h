#pragma once
#include <sys/types.h>
struct stat {
    off_t st_size;
    mode_t st_mode;
};
int stat(const char *path, struct stat *st);
int mkdir(const char *path, mode_t mode);
#define S_IFMT   0170000
#define S_IFDIR  0040000
#define S_ISDIR(m) (((m) & S_IFMT) == S_IFDIR)