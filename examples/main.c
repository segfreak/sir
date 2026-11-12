#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <limits.h>

/* defined in `fib.sir` */
int32_t fib(int32_t);

static void showhelp(const char *execname) {
    printf("Usage: %s NUMBER\n", execname);
}

int main(int argc, char **argv) {
    if (argc != 2) {
        showhelp(argv[0]);
        return EXIT_FAILURE;
    }

    errno = 0;
    char *end = NULL;
    long value = strtol(argv[1], &end, 10);

    if (errno == ERANGE || end == argv[1] || *end != '\0' ||
        value < 0 || value > INT32_MAX) {
        fprintf(stderr, "%s: error: invalid number: '%s'\n", argv[0], argv[1]);
        showhelp(argv[0]);
        return EXIT_FAILURE;
    }

    int32_t n = (int32_t)value;

    printf("%d\n", fib(n));

    return EXIT_SUCCESS;
}
