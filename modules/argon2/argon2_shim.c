/*
 * argon2_shim.c - puente FFI "sin punteros" entre CLS y libargon2.
 *
 * Se compila junto con las fuentes de libargon2 (argon2.c, core.c, blake2b.c,
 * encoding.c, thread.c), por lo que no hay una DLL de libargon2 separada.
 *
 * API CLS:
 *   o_hash(password, salt_hex, t, m_kib, p, hash_len) -> encoded
 *   o_verify(encoded, password) -> 1/0
 *   o_needs_rehash(encoded, t, m_kib, p) -> 1/0
 *   o_error() -> ultimo error de libargon2
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "argon2.h"

#ifdef _WIN32
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((visibility("default")))
#endif

static char g_error[256];
static char g_encoded[512];

static void set_error(const char *msg) {
    snprintf(g_error, sizeof(g_error), "%s", msg ? msg : "");
}

static int hex_value(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

static int decode_hex(const char *hex, unsigned char *out, size_t out_cap, size_t *out_len) {
    size_t n = hex ? strlen(hex) : 0;
    if (n == 0 || (n % 2) != 0 || n / 2 > out_cap) return -1;
    for (size_t i = 0; i < n / 2; i++) {
        int hi = hex_value(hex[i * 2]);
        int lo = hex_value(hex[i * 2 + 1]);
        if (hi < 0 || lo < 0) return -1;
        out[i] = (unsigned char)((hi << 4) | lo);
    }
    *out_len = n / 2;
    return 0;
}

EXPORT const char *o_error(void) {
    return g_error;
}

EXPORT const char *o_hash(
    const char *password,
    const char *salt_hex,
    int32_t t_cost,
    int32_t m_kib,
    int32_t parallelism,
    int32_t hash_len
) {
    if (!password || !salt_hex) { set_error("password/salt nulos"); return ""; }
    if (t_cost < 1 || t_cost > 16 || m_kib < 8 || m_kib > 1048576 ||
        parallelism < 1 || parallelism > 16 || hash_len < 16 || hash_len > 64) {
        set_error("parametros Argon2 fuera de rango");
        return "";
    }
    unsigned char salt[64];
    size_t salt_len = 0;
    if (decode_hex(salt_hex, salt, sizeof(salt), &salt_len) != 0 || salt_len < 8) {
        set_error("salt hex invalido");
        return "";
    }
    size_t encoded_len = argon2_encodedlen(
        (uint32_t)t_cost, (uint32_t)m_kib, (uint32_t)parallelism,
        (uint32_t)salt_len, (uint32_t)hash_len, Argon2_id
    );
    if (encoded_len > sizeof(g_encoded)) {
        set_error("buffer de encoded insuficiente");
        return "";
    }
    int rc = argon2id_hash_encoded(
        (uint32_t)t_cost, (uint32_t)m_kib, (uint32_t)parallelism,
        password, strlen(password), salt, salt_len,
        (size_t)hash_len, g_encoded, sizeof(g_encoded)
    );
    if (rc != ARGON2_OK) {
        set_error(argon2_error_message(rc));
        return "";
    }
    set_error("");
    return g_encoded;
}

EXPORT int32_t o_verify(const char *encoded, const char *password) {
    if (!encoded || !password) return 0;
    int rc = argon2id_verify(encoded, password, strlen(password));
    if (rc == ARGON2_OK) return 1;
    set_error(argon2_error_message(rc));
    return 0;
}

EXPORT int32_t o_needs_rehash(
    const char *encoded, int32_t t_cost, int32_t m_kib, int32_t parallelism
) {
    if (!encoded) return 1;
    char prefix[128];
    snprintf(prefix, sizeof(prefix), "$argon2id$v=19$m=%d,t=%d,p=%d$",
             (int)m_kib, (int)t_cost, (int)parallelism);
    return strncmp(encoded, prefix, strlen(prefix)) == 0 ? 0 : 1;
}
