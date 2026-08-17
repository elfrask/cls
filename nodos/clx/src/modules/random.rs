use cls_runtime::value::{FunValue, Value};
use std::collections::HashMap;

/// LCG sembrado una vez con entropía (paridad con math_random del JIT).
fn rng_next_u64() -> u64 {
    use std::sync::OnceLock;
    static RNG_STATE: OnceLock<std::sync::Mutex<u64>> = OnceLock::new();
    let state = RNG_STATE.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seed = (nanos as u64) ^ ((std::process::id() as u64) << 32);
        std::sync::Mutex::new(seed | 1)
    });
    let mut s = state.lock().unwrap();
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *s
}

fn rng_next_f64() -> f64 {
    (rng_next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

pub fn module() -> Value {
    let mut m = HashMap::new();
    m.insert("random".into(), Value::Fun(FunValue::new_native("random", vec![], |_| {
        Ok(Value::Float(rng_next_f64()))
    })));
    m.insert("int".into(), Value::Fun(FunValue::new_native("int", vec!["min".into(), "max".into()], |a| {
        let min = match a.first() { Some(Value::Int(i)) => *i, _ => 0 };
        let max = match a.get(1) { Some(Value::Int(i)) => *i, _ => min };
        if max <= min {
            return Ok(Value::Int(min));
        }
        let range = (max - min + 1) as u64;
        Ok(Value::Int(min + (rng_next_u64() % range) as i64))
    })));
    m.insert("float".into(), Value::Fun(FunValue::new_native("float", vec!["min".into(), "max".into()], |a| {
        let min = match a.first() { Some(Value::Float(f)) => *f, Some(Value::Int(i)) => *i as f64, _ => 0.0 };
        let max = match a.get(1) { Some(Value::Float(f)) => *f, Some(Value::Int(i)) => *i as f64, _ => min };
        Ok(Value::Float(min + rng_next_f64() * (max - min)))
    })));
    m.insert("uuid".into(), Value::Fun(FunValue::new_native("uuid", vec![], |_| {
        let mut b = [0u8; 16];
        for chunk in b.chunks_mut(8) {
            let v = rng_next_u64().to_le_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
        b[6] = (b[6] & 0x0f) | 0x40;
        b[8] = (b[8] & 0x3f) | 0x80;
        let h = b.iter().map(|x| format!("{:02x}", x)).collect::<Vec<_>>();
        Ok(Value::String(format!("{}-{}-{}-{}-{}", h[0..4].concat(), h[4..6].concat(), h[6..8].concat(), h[8..10].concat(), h[10..16].concat())))
    })));
    Value::Record(m)
}
