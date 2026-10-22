#!/usr/bin/env python3
# server.py - Servidor DNS AUTORITATIVO independiente (UDP + TCP) para la
# prueba #6. Es el ORACULO externo en vivo: el runner CLS le consulta y compara
# lo que decodifica con los valores esperados de la zona.
#
# Independiente de CLS: usa `dns_indep.py` (Python puro, sin dnslib).
#   - UDP 127.0.0.1:<port>: responde la zona; para `big.example.test` responde
#     TC=1 (respuesta truncada) -> fuerza retry TCP.
#   - TCP 127.0.0.1:<port>: framing [len:u16 BE][msg]; responde completo.
#   - Registra cada query recibida (hex, una por linea) en --log.
#
# Uso:
#   python server.py --port 15353 --log examples/dns/out/queries.log

import argparse
import os
import socket
import struct
import sys
import threading

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from dns_indep import parse_message, build_response, build_tc_response  # noqa: E402
from zone import load_zone  # noqa: E402

LOG_PATH = None
LOG_LOCK = threading.Lock()


def log_query(data):
    if not LOG_PATH:
        return
    with LOG_LOCK:
        with open(LOG_PATH, "a", encoding="utf-8") as f:
            f.write(data.hex() + "\n")


def should_truncate(data):
    try:
        q = parse_message(data)
    except Exception:
        return False
    return q["qname"].lower() == "big.example.test"


def recv_exact(conn, n):
    buf = bytearray()
    while len(buf) < n:
        chunk = conn.recv(n - len(buf))
        if not chunk:
            return None
        buf += chunk
    return bytes(buf)


def udp_loop(sock, zone):
    while True:
        try:
            data, addr = sock.recvfrom(4096)
        except OSError:
            break
        log_query(data)
        if should_truncate(data):
            resp = build_tc_response(data)
        else:
            resp = build_response(data, zone)
        try:
            sock.sendto(resp, addr)
        except OSError:
            pass


def tcp_conn(conn, addr, zone):
    try:
        hdr = recv_exact(conn, 2)
        if not hdr:
            return
        n = (hdr[0] << 8) | hdr[1]
        data = recv_exact(conn, n)
        if data is None:
            return
        log_query(data)
        resp = build_response(data, zone)  # respuesta completa por TCP
        conn.sendall(struct.pack("!H", len(resp)) + resp)
    except OSError:
        pass
    finally:
        try:
            conn.close()
        except OSError:
            pass


def tcp_loop(sock, zone):
    while True:
        try:
            conn, addr = sock.accept()
        except OSError:
            break
        threading.Thread(target=tcp_conn, args=(conn, addr, zone), daemon=True).start()


def main(argv):
    global LOG_PATH
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=15353)
    ap.add_argument("--log", default=None)
    ap.add_argument("--zone", default=None)
    args = ap.parse_args(argv[1:])

    LOG_PATH = args.log
    if LOG_PATH and os.path.exists(LOG_PATH):
        os.remove(LOG_PATH)

    zone = load_zone(args.zone)

    us = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    us.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    us.bind((args.host, args.port))

    ts = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    ts.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    ts.bind((args.host, args.port))
    ts.listen(8)

    print("dns oracle listening on %s:%d (udp+tcp)" % (args.host, args.port), flush=True)

    tu = threading.Thread(target=udp_loop, args=(us, zone), daemon=True)
    tt = threading.Thread(target=tcp_loop, args=(ts, zone), daemon=True)
    tu.start()
    tt.start()
    tu.join()
    tt.join()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
