use cls_runtime::value::{FunValue, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::net::TcpListener as StdTcpListener;
use std::net::TcpStream as StdTcpStream;

/// Estado global de los sockets del mÃ³dulo `net` (mismo contrato que el host
/// del JIT en `cls-jit/src/host.rs`). Handle i64 incremental por listener/stream.
static NET_NEXT_ID: AtomicI64 = AtomicI64::new(1);
static NET_LISTENERS: OnceLock<Mutex<HashMap<i64, StdTcpListener>>> = OnceLock::new();
static NET_STREAMS: OnceLock<Mutex<HashMap<i64, StdTcpStream>>> = OnceLock::new();
static NET_LAST_ERROR: Mutex<String> = Mutex::new(String::new());

fn net_listeners() -> &'static Mutex<HashMap<i64, StdTcpListener>> {
    NET_LISTENERS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn net_streams() -> &'static Mutex<HashMap<i64, StdTcpStream>> {
    NET_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn net_set_error(msg: &str) {
    if let Ok(mut e) = NET_LAST_ERROR.lock() {
        *e = msg.to_string();
    }
}

fn net_listen(port: i64) -> i64 {
    let addr = format!("127.0.0.1:{}", port);
    match StdTcpListener::bind(&addr) {
        Ok(listener) => {
            let id = NET_NEXT_ID.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut l) = net_listeners().lock() {
                l.insert(id, listener);
            }
            net_set_error("");
            id
        }
        Err(e) => {
            net_set_error(&format!("listen {}: {}", addr, e));
            0
        }
    }
}

fn net_accept(handle: i64) -> i64 {
    let listener = match net_listeners().lock() {
        Ok(l) => l.get(&handle).map(|l| l.try_clone().ok()).flatten(),
        Err(_) => None,
    };
    let listener = match listener {
        Some(l) => l,
        None => {
            net_set_error("accept: listener no existe");
            return 0;
        }
    };
    match listener.accept() {
        Ok((stream, _)) => {
            let id = NET_NEXT_ID.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut s) = net_streams().lock() {
                s.insert(id, stream);
            }
            net_set_error("");
            id
        }
        Err(e) => {
            net_set_error(&format!("accept: {}", e));
            0
        }
    }
}

fn net_recv(sock: i64, max: i64) -> String {
    use std::io::Read;
    let max = max.clamp(1, 1_000_000) as usize;
    let mut buf = vec![0u8; max];
    let stream = match net_streams().lock() {
        Ok(s) => s.get(&sock).and_then(|s| s.try_clone().ok()),
        Err(_) => None,
    };
    let mut stream = match stream {
        Some(s) => s,
        None => {
            net_set_error("recv: socket no existe");
            return String::new();
        }
    };
    match stream.read(&mut buf) {
        Ok(0) => {
            net_set_error("");
            String::new()
        }
        Ok(n) => {
            net_set_error("");
            String::from_utf8_lossy(&buf[..n]).into_owned()
        }
        Err(e) => {
            net_set_error(&format!("recv: {}", e));
            String::new()
        }
    }
}

fn net_send(sock: i64, data: String) -> i64 {
    use std::io::Write;
    let stream = match net_streams().lock() {
        Ok(s) => s.get(&sock).and_then(|s| s.try_clone().ok()),
        Err(_) => None,
    };
    let mut stream = match stream {
        Some(s) => s,
        None => {
            net_set_error("send: socket no existe");
            return 0;
        }
    };
    match stream.write(data.as_bytes()) {
        Ok(n) => {
            net_set_error("");
            let _ = stream.flush();
            n as i64
        }
        Err(e) => {
            net_set_error(&format!("send: {}", e));
            0
        }
    }
}

fn net_close(handle: i64) -> i64 {
    let mut removed = false;
    if let Ok(mut s) = net_streams().lock() {
        if s.remove(&handle).is_some() {
            removed = true;
        }
    }
    if !removed {
        if let Ok(mut l) = net_listeners().lock() {
            let _ = l.remove(&handle);
        }
    }
    net_set_error("");
    0
}

fn net_last_error() -> String {
    match NET_LAST_ERROR.lock() {
        Ok(e) => e.clone(),
        Err(_) => String::new(),
    }
}

pub fn module() -> Value {
    let mut m = HashMap::new();
    m.insert("listen".into(), Value::Fun(FunValue::new_native("listen", vec!["port".into()], |a| {
        let port = match a.first() { Some(Value::Int(i)) => *i, _ => 0 };
        Ok(Value::Int(net_listen(port)))
    })));
    m.insert("accept".into(), Value::Fun(FunValue::new_native("accept", vec!["handle".into()], |a| {
        let handle = match a.first() { Some(Value::Int(i)) => *i, _ => 0 };
        Ok(Value::Int(net_accept(handle)))
    })));
    m.insert("recv".into(), Value::Fun(FunValue::new_native("recv", vec!["sock".into(), "max".into()], |a| {
        let sock = match a.first() { Some(Value::Int(i)) => *i, _ => 0 };
        let max = match a.get(1) { Some(Value::Int(i)) => *i, _ => 1024 };
        Ok(Value::String(net_recv(sock, max)))
    })));
    m.insert("send".into(), Value::Fun(FunValue::new_native("send", vec!["sock".into(), "data".into()], |a| {
        let sock = match a.first() { Some(Value::Int(i)) => *i, _ => 0 };
        let data = match a.get(1) { Some(Value::String(s)) => s.clone(), _ => String::new() };
        Ok(Value::Int(net_send(sock, data)))
    })));
    m.insert("close".into(), Value::Fun(FunValue::new_native("close", vec!["handle".into()], |a| {
        let handle = match a.first() { Some(Value::Int(i)) => *i, _ => 0 };
        Ok(Value::Int(net_close(handle)))
    })));
    m.insert("lastError".into(), Value::Fun(FunValue::new_native("lastError", vec![], |_| {
        Ok(Value::String(net_last_error()))
    })));
    Value::Record(m)
}
