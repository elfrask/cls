/*
 * socket_shim.c - Puente FFI "sin punteros" para sockets TCP persistentes
 * (modulo `modules/socket`, CLS + FFI). Ladrillo de la stdlib autocontenida
 * (examples/patrones.md §2.2 y §11): la unica puerta de escape a C son las
 * syscalls de sockets, envueltas aqui.
 *
 * A diferencia de `modules/net` (que aplica framing DNS-over-TCP), este shim
 * expone un stream TCP crudo y BIDIRECCIONAL: se mantiene la conexion abierta y
 * el llamador puede enviar y recibir cuantas veces quiera (HTTP keep-alive,
 * WebSocket, protocolos propios...).
 *
 * REGLA: el codigo CLS NUNCA manipula punteros. Todo cruza como:
 *   - handles de socket  -> enteros (int64_t)
 *   - payloads (bytes)   -> CArray CLS [cap:i64][len:i64][elems*8] (un byte por
 *                           elemento i64; NO se usa CString porque los datos
 *                           pueden contener bytes NUL)
 *   - direcciones        -> CString (char*, copiado a la memoria del modulo)
 * El shim NO expone ninguna direccion de memoria al lenguaje.
 *
 * Compilacion (Windows, MinGW-w64 x64; ver build.ps1):
 *   gcc -shared -O2 -o modules/socket/socket_shim.dll modules/socket/socket_shim.c -lws2_32
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
#define SK_CLOSE(s) closesocket(s)
#define SK_BAD INVALID_SOCKET
#else
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/select.h>
typedef int sock_t;
#define SK_CLOSE(s) close(s)
#define SK_BAD (-1)
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

static size_t carray_len(int64_t p) {
    int64_t len = 0;
    if (!p) return 0;
    memcpy(&len, (const void *)(uintptr_t)(p + 8), 8);
    return len < 0 ? 0 : (size_t)len;
}

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

/* Espera a que el socket sea legible; 1 legible (o EOF), 0 timeout/error. */
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

static int wait_writable(sock_t s, int timeout_ms) {
    fd_set fds;
    FD_ZERO(&fds);
    FD_SET(s, &fds);
    struct timeval tv;
    tv.tv_sec = timeout_ms / 1000;
    tv.tv_usec = (timeout_ms % 1000) * 1000;
#ifdef _WIN32
    int r = select(0, NULL, &fds, NULL, &tv);
#else
    int r = select((int)s + 1, NULL, &fds, NULL, &tv);
#endif
    return r > 0 ? 1 : 0;
}

static void set_nonblocking(sock_t s, int on) {
#ifdef _WIN32
    unsigned long v = on ? 1 : 0;
    ioctlsocket(s, FIONBIO, &v);
#else
    int fl = fcntl(s, F_GETFL, 0);
    if (on) fl |= O_NONBLOCK; else fl &= ~O_NONBLOCK;
    fcntl(s, F_SETFL, fl);
#endif
}

static char *dup_str(const char *s) {
    size_t n = s ? strlen(s) : 0;
    char *out = (char *)malloc(n + 1);
    if (!out) return NULL;
    if (n) memcpy(out, s, n);
    out[n] = '\0';
    return out;
}

/* ------------------------------------------------------------------ */
/* Conexion (resuelve con getaddrinfo; soporta timeout).               */
/* ------------------------------------------------------------------ */

static int64_t do_connect(const char *host, int32_t port, int32_t timeout_ms) {
    if (!host || !host[0]) return -2;
    ensure_init();
    char portstr[16];
    snprintf(portstr, sizeof(portstr), "%d", (int)port);
    struct addrinfo hints, *res = NULL;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    if (getaddrinfo(host, portstr, &hints, &res) != 0 || !res) return -2;
    sock_t s = socket(res->ai_family, res->ai_socktype, res->ai_protocol);
    if (s == SK_BAD) { freeaddrinfo(res); return -1; }

    if (timeout_ms > 0) {
        set_nonblocking(s, 1);
        int rc = connect(s, res->ai_addr, (int)res->ai_addrlen);
        if (rc != 0) {
            if (!wait_writable(s, timeout_ms)) {
                SK_CLOSE(s); freeaddrinfo(res); return -4; /* timeout */
            }
            int err = 0;
#ifdef _WIN32
            int elen = (int)sizeof(err);
#else
            socklen_t elen = sizeof(err);
#endif
            getsockopt(s, SOL_SOCKET, SO_ERROR, (char *)&err, &elen);
            if (err != 0) { SK_CLOSE(s); freeaddrinfo(res); return -3; }
        }
        set_nonblocking(s, 0);
    } else {
        if (connect(s, res->ai_addr, (int)res->ai_addrlen) != 0) {
            SK_CLOSE(s); freeaddrinfo(res); return -3;
        }
    }
    freeaddrinfo(res);
    int one = 1;
    setsockopt(s, IPPROTO_TCP, TCP_NODELAY, (char *)&one, sizeof(one));
    return (int64_t)s;
}

DL int64_t sk_connect(const char *host, int32_t port) {
    return do_connect(host, port, 0);
}

DL int64_t sk_connect_timeout(const char *host, int32_t port, int32_t timeout_ms) {
    return do_connect(host, port, timeout_ms);
}

/* ------------------------------------------------------------------ */
/* Envio / recepcion.                                                   */
/* ------------------------------------------------------------------ */

/* Envia TODOS los bytes del CArray. Devuelve bytes escritos o -1. */
DL int64_t sk_send(int64_t h, int64_t buf) {
    size_t n = 0;
    unsigned char *tmp = carray_to_buf(buf, &n);
    if (!tmp) return -1;
    size_t sent = 0;
    while (sent < n) {
        int r = send((sock_t)h, (const char *)tmp + sent, (int)(n - sent), 0);
        if (r <= 0) { free(tmp); return -1; }
        sent += (size_t)r;
    }
    free(tmp);
    return (int64_t)n;
}

/* Envia "close" del lado escritura (half-close) para señalar EOF al peer. */
DL int64_t sk_shutdown_write(int64_t h) {
#ifdef _WIN32
    return (int64_t)shutdown((sock_t)h, SD_SEND);
#else
    return (int64_t)shutdown((sock_t)h, SHUT_WR);
#endif
}

/* Recibe hasta maxLen bytes. CArray vacio en timeout/EOF/error. */
DL int64_t sk_recv(int64_t h, int32_t maxLen, int32_t timeout_ms) {
    int cap = maxLen > 0 ? maxLen : 1;
    unsigned char *tmp = (unsigned char *)malloc((size_t)cap);
    if (!tmp) return 0;
    if (!wait_readable((sock_t)h, timeout_ms)) { free(tmp); return 0; }
    int r = recv((sock_t)h, (char *)tmp, cap, 0);
    if (r <= 0) { free(tmp); return 0; }
    int64_t arr = pack_bytes(tmp, (size_t)r);
    free(tmp);
    return arr;
}

/* Lee EXACTAMENTE n bytes (para Content-Length, cabeceras, etc.). Vacio si no
 * se pudieron leer todos (timeout/EOF). */
DL int64_t sk_recv_exact(int64_t h, int32_t n, int32_t timeout_ms) {
    if (n < 0) return 0;
    unsigned char *tmp = (unsigned char *)malloc(n ? (size_t)n : 1);
    if (!tmp) return 0;
    size_t got = 0;
    while (got < (size_t)n) {
        if (!wait_readable((sock_t)h, timeout_ms)) { free(tmp); return 0; }
        int r = recv((sock_t)h, (char *)tmp + got, (int)((size_t)n - got), 0);
        if (r <= 0) { free(tmp); return 0; }
        got += (size_t)r;
    }
    int64_t arr = pack_bytes(tmp, (size_t)n);
    free(tmp);
    return arr;
}

/* Devuelve 1 si hay datos (o EOF) listos, 0 si expiro el timeout. */
DL int32_t sk_readable(int64_t h, int32_t timeout_ms) {
    return wait_readable((sock_t)h, timeout_ms) ? 1 : 0;
}

/* Ajusta el timeout de recv/envio a nivel de socket (SO_RCVTIMEO/SO_SNDTIMEO). */
DL int64_t sk_set_timeout(int64_t h, int32_t ms) {
    struct timeval tv;
    tv.tv_sec = ms / 1000;
    tv.tv_usec = (ms % 1000) * 1000;
    setsockopt((sock_t)h, SOL_SOCKET, SO_RCVTIMEO, (char *)&tv, sizeof(tv));
    setsockopt((sock_t)h, SOL_SOCKET, SO_SNDTIMEO, (char *)&tv, sizeof(tv));
    return 0;
}

DL int64_t sk_close(int64_t h) {
    return (int64_t)SK_CLOSE((sock_t)h);
}

/* Ultimo error del sistema (texto). "" si no hay. */
DL char *sk_last_error(void) {
#ifdef _WIN32
    int e = WSAGetLastError();
    static char buf[64];
    snprintf(buf, sizeof(buf), "wsa:%d", e);
    return dup_str(buf);
#else
    return dup_str(strerror(errno));
#endif
}
