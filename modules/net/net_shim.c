/*
 * net_shim.c - Puente FFI "sin punteros" para transporte UDP/TCP del modulo
 * `modules/net` (CLS + FFI). Ladrillo de la stdlib autocontenida
 * (examples/patrones.md §2.2 y §11): la unica puerta de escape a C son las
 * syscalls de sockets, envueltas aqui.
 *
 * REGLA: el codigo CLS NUNCA manipula punteros. Todo cruza como:
 *   - handles de socket  -> enteros (int64_t)
 *   - payloads (bytes)   -> CArray CLS [cap:i64][len:i64][elems*8] (un byte por
 *                           elemento i64; NO se usa CString porque los paquetes
 *                           DNS contienen bytes NUL)
 *   - direcciones        -> CString (char*, copiado a la memoria del modulo)
 * El shim NO expone ninguna direccion de memoria al lenguaje.
 *
 * Compilacion (Windows, MinGW-w64 x64; ver build.ps1):
 *   gcc -shared -O2 -o modules/net/net_shim.dll modules/net/net_shim.c -lws2_32
 * POSIX (Linux/macOS): libc (sin -lws2_32).
 *
 * Este shim es FFI legitimo (no es un `host_call`, no es una feature del host).
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
typedef SOCKET sock_t;
#define NET_CLOSE(s) closesocket(s)
#define NET_BAD INVALID_SOCKET
#define NET_PLACEHOLDER 0
#else
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <unistd.h>
#include <errno.h>
#include <sys/select.h>
typedef int sock_t;
#define NET_CLOSE(s) close(s)
#define NET_BAD (-1)
#define NET_PLACEHOLDER 0
#endif

#ifdef _WIN32
#define DL __declspec(dllexport)
#else
#define DL __attribute__((visibility("default")))
#endif

/* ------------------------------------------------------------------ */
/* Arranque (Winsock) y utilidades de layout CArray.                    */
/* ------------------------------------------------------------------ */

static int g_wsa_started = 0;

static void ensure_init(void) {
#ifdef _WIN32
    if (!g_wsa_started) {
        WSADATA w;
        WSAStartup(MAKEWORD(2, 2), &w);
        g_wsa_started = 1;
    }
#endif
}

/* Empaqueta bytes crudos como CArray CLS [cap:i64][len:i64][elems*8]. */
static int64_t pack_bytes(const unsigned char *raw, size_t n) {
    unsigned char *arr = (unsigned char *)malloc(16 + (n ? n : 1) * 8);
    if (!arr) return 0;
    int64_t cap = (int64_t)n, len = (int64_t)n;
    memcpy(arr, &cap, 8);
    memcpy(arr + 8, &len, 8);
    for (size_t i = 0; i < n; i++) {
        int64_t v = raw[i] & 0xff;
        memcpy(arr + 16 + i * 8, &v, 8);
    }
    return (int64_t)(uintptr_t)arr;
}

/* len del CArray [cap][len][elems]. */
static size_t carray_len(int64_t p) {
    int64_t len = 0;
    if (!p) return 0;
    memcpy(&len, (const void *)(uintptr_t)(p + 8), 8);
    return len < 0 ? 0 : (size_t)len;
}

/* Copia los bytes del CArray a un buffer C temporal. */
static unsigned char *carray_to_buf(int64_t p, size_t *out_n) {
    size_t n = carray_len(p);
    unsigned char *tmp = (unsigned char *)malloc(n ? n : 1);
    if (!tmp) { *out_n = 0; return NULL; }
    const unsigned char *src = (const unsigned char *)(uintptr_t)p;
    for (size_t i = 0; i < n; i++) {
        int64_t v = 0;
        memcpy(&v, src + 16 + i * 8, 8);
        tmp[i] = (unsigned char)(v & 0xff);
    }
    *out_n = n;
    return tmp;
}

/* Espera a que el socket sea legible; devuelve 1 legible, 0 timeout/error. */
static int wait_readable(sock_t s, int timeout_ms) {
    if (timeout_ms < 0) return 1;
    fd_set fds;
    FD_ZERO(&fds);
    FD_SET(s, &fds);
    struct timeval tv;
    tv.tv_sec = timeout_ms / 1000;
    tv.tv_usec = (timeout_ms % 1000) * 1000;
#ifdef _WIN32
    int r = select(0, &fds, NULL, NULL, &tv);
#else
    int r = select((int)s + 1, &fds, NULL, NULL, &tv);
#endif
    return r > 0 ? 1 : 0;
}

/* ------------------------------------------------------------------ */
/* Resolucion de nombres (envuelve getaddrinfo; NO expone sockaddr).    */
/* ------------------------------------------------------------------ */

static char *dup_str(const char *s) {
    size_t n = s ? strlen(s) : 0;
    char *out = (char *)malloc(n + 1);
    if (!out) return NULL;
    if (n) memcpy(out, s, n);
    out[n] = '\0';
    return out;
}

DL char *net_resolve(const char *host) {
    ensure_init();
    if (!host || !host[0]) return dup_str("");
    struct addrinfo hints, *res = NULL;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    if (getaddrinfo(host, NULL, &hints, &res) != 0 || !res) return dup_str("");
    char buf[INET_ADDRSTRLEN];
    struct sockaddr_in *si = (struct sockaddr_in *)res->ai_addr;
    const char *p = inet_ntop(AF_INET, &si->sin_addr, buf, sizeof(buf));
    freeaddrinfo(res);
    return dup_str(p ? p : "");
}

static int parse_addr(const char *ip, int32_t port, struct sockaddr_in *out) {
    memset(out, 0, sizeof(*out));
    out->sin_family = AF_INET;
    out->sin_port = htons((uint16_t)port);
    if (inet_pton(AF_INET, ip, &out->sin_addr) != 1) return 0;
    return 1;
}

/* ------------------------------------------------------------------ */
/* UDP: u_open / u_send / u_recv / u_close                             */
/* ------------------------------------------------------------------ */

DL int64_t u_open(const char *ip, int32_t port) {
    ensure_init();
    sock_t s = socket(AF_INET, SOCK_DGRAM, 0);
    if (s == NET_BAD) return -1;
    struct sockaddr_in a;
    if (!parse_addr(ip, port, &a)) { NET_CLOSE(s); return -2; }
    if (connect(s, (struct sockaddr *)&a, (int)sizeof(a)) != 0) {
        NET_CLOSE(s);
        return -3;
    }
    return (int64_t)s;
}

DL int64_t u_send(int64_t h, int64_t buf) {
    size_t n = 0;
    unsigned char *tmp = carray_to_buf(buf, &n);
    if (!tmp) return -1;
    int r = send((sock_t)h, (const char *)tmp, (int)n, 0);
    free(tmp);
    return (int64_t)r;
}

/* timeoutMs < 0 = bloqueante. Devuelve CArray (vacio en timeout/error). */
DL int64_t u_recv(int64_t h, int32_t maxLen, int32_t timeoutMs) {
    unsigned char *tmp = (unsigned char *)malloc(maxLen > 0 ? (size_t)maxLen : 1);
    if (!tmp) return 0;
    if (!wait_readable((sock_t)h, timeoutMs)) { free(tmp); return 0; }
    int r = recv((sock_t)h, (char *)tmp, maxLen, 0);
    if (r <= 0) { free(tmp); return 0; }
    int64_t arr = pack_bytes(tmp, (size_t)r);
    free(tmp);
    return arr;
}

DL int64_t u_close(int64_t h) {
    return (int64_t)NET_CLOSE((sock_t)h);
}

/* ------------------------------------------------------------------ */
/* TCP: t_open / t_send / t_recv / t_close                             */
/* Framing DNS-over-TCP: [len:u16 BE][payload].                        */
/* ------------------------------------------------------------------ */

DL int64_t t_open(const char *ip, int32_t port) {
    ensure_init();
    sock_t s = socket(AF_INET, SOCK_STREAM, 0);
    if (s == NET_BAD) return -1;
    struct sockaddr_in a;
    if (!parse_addr(ip, port, &a)) { NET_CLOSE(s); return -2; }
    if (connect(s, (struct sockaddr *)&a, (int)sizeof(a)) != 0) {
        NET_CLOSE(s);
        return -3;
    }
    return (int64_t)s;
}

/* Envia [len:u16 BE] + payload. Devuelve bytes de payload escritos o -1. */
DL int64_t t_send(int64_t h, int64_t buf) {
    size_t n = 0;
    unsigned char *tmp = carray_to_buf(buf, &n);
    if (!tmp) return -1;
    if (n > 0xffff) { free(tmp); return -2; }
    unsigned char *msg = (unsigned char *)malloc(n + 2);
    if (!msg) { free(tmp); return -1; }
    msg[0] = (unsigned char)((n >> 8) & 0xff);
    msg[1] = (unsigned char)(n & 0xff);
    memcpy(msg + 2, tmp, n);
    free(tmp);
    size_t total = n + 2, sent = 0;
    while (sent < total) {
        int r = send((sock_t)h, (const char *)msg + sent, (int)(total - sent), 0);
        if (r <= 0) { free(msg); return -1; }
        sent += (size_t)r;
    }
    free(msg);
    return (int64_t)n;
}

static int read_exact(sock_t s, unsigned char *dst, size_t n, int timeout_ms) {
    size_t got = 0;
    while (got < n) {
        if (!wait_readable(s, timeout_ms)) return -1;
        int r = recv(s, (char *)dst + got, (int)(n - got), 0);
        if (r <= 0) return -1;
        got += (size_t)r;
    }
    return 0;
}

/* Lee el prefijo u16 BE y luego el payload. Devuelve CArray con el payload. */
DL int64_t t_recv(int64_t h, int32_t maxLen, int32_t timeoutMs) {
    (void)maxLen;
    unsigned char hdr[2];
    if (read_exact((sock_t)h, hdr, 2, timeoutMs) != 0) return 0;
    size_t mlen = ((size_t)hdr[0] << 8) | (size_t)hdr[1];
    if (mlen == 0) return pack_bytes((const unsigned char *)"", 0);
    unsigned char *tmp = (unsigned char *)malloc(mlen);
    if (!tmp) return 0;
    if (read_exact((sock_t)h, tmp, mlen, timeoutMs) != 0) { free(tmp); return 0; }
    int64_t arr = pack_bytes(tmp, mlen);
    free(tmp);
    return arr;
}

DL int64_t t_close(int64_t h) {
    return (int64_t)NET_CLOSE((sock_t)h);
}
