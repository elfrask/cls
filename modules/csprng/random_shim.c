/*
 * random_shim.c - CSPRNG portable para CLS (sin punteros en la API CLS).
 *
 * Windows: BCryptGenRandom (RNG del sistema).
 * Linux/macOS: /dev/urandom con lectura completa.
 *
 * Devuelve un CArray empaquetado como el resto de shims del repo:
 *   [cap: i64][len: i64][elem0: i64]...
 * donde cada elemento es un byte 0..255. El CLS lo recibe como int[].
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
#include <bcrypt.h>
#define EXPORT __declspec(dllexport)
#else
#include <errno.h>
#include <fcntl.h>
#include <unistd.h>
#define EXPORT __attribute__((visibility("default")))
#endif

static int64_t pack_bytes(const unsigned char *raw, size_t n) {
    unsigned char *arr = (unsigned char *)malloc(16 + (n ? n : 1) * 8);
    if (!arr) return 0;
    int64_t cap = (int64_t)n;
    int64_t len = (int64_t)n;
    memcpy(arr, &cap, 8);
    memcpy(arr + 8, &len, 8);
    for (size_t i = 0; i < n; i++) {
        int64_t v = (int64_t)(raw[i] & 0xff);
        memcpy(arr + 16 + i * 8, &v, 8);
    }
    return (int64_t)(uintptr_t)arr;
}

EXPORT int64_t o_random_bytes(int32_t n) {
    if (n <= 0 || n > 1048576) return 0;
    unsigned char *buf = (unsigned char *)malloc((size_t)n);
    if (!buf) return 0;

#ifdef _WIN32
    NTSTATUS st = BCryptGenRandom(
        NULL, buf, (ULONG)n, BCRYPT_USE_SYSTEM_PREFERRED_RNG
    );
    if (st != 0) {
        free(buf);
        return 0;
    }
#else
    int fd = open("/dev/urandom", O_RDONLY);
    if (fd < 0) {
        free(buf);
        return 0;
    }
    size_t got = 0;
    while (got < (size_t)n) {
        ssize_t r = read(fd, buf + got, (size_t)n - got);
        if (r <= 0) {
            if (r < 0 && errno == EINTR) continue;
            close(fd);
            free(buf);
            return 0;
        }
        got += (size_t)r;
    }
    close(fd);
#endif

    int64_t out = pack_bytes(buf, (size_t)n);
    free(buf);
    return out;
}
