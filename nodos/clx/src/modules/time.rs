use cls_runtime::value::{FunValue, Value};
use std::collections::HashMap;

/// Descompone un epoch (segundos) en fecha/hora UTC.
fn epoch_fields(secs: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    // Algoritmo civil (Howard Hinnant).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, hour, minute, second)
}

fn pad2(n: i64) -> String {
    if n < 10 {
        format!("0{}", n)
    } else {
        n.to_string()
    }
}

fn epoch_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn module() -> Value {
    let mut m = HashMap::new();
    m.insert("now".into(), Value::Fun(FunValue::new_native("now", vec![], |_| {
        Ok(Value::Int(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)))
    })));
    m.insert("seconds".into(), Value::Fun(FunValue::new_native("seconds", vec![], |_| {
        Ok(Value::Int(epoch_now()))
    })));
    m.insert("iso".into(), Value::Fun(FunValue::new_native("iso", vec![], |_| {
        let (y, mo, d, h, mi, s) = epoch_fields(epoch_now());
        Ok(Value::String(format!("{:04}-{}-{}T{}:{}:{}Z", y, pad2(mo), pad2(d), pad2(h), pad2(mi), pad2(s))))
    })));
    m.insert("date".into(), Value::Fun(FunValue::new_native("date", vec![], |_| {
        let (y, mo, d, _, _, _) = epoch_fields(epoch_now());
        Ok(Value::String(format!("{:04}-{}-{}", y, pad2(mo), pad2(d))))
    })));
    m.insert("clock".into(), Value::Fun(FunValue::new_native("clock", vec![], |_| {
        let (_, _, _, h, mi, s) = epoch_fields(epoch_now());
        Ok(Value::String(format!("{}:{}:{}", pad2(h), pad2(mi), pad2(s))))
    })));
    m.insert("year".into(), Value::Fun(FunValue::new_native("year", vec![], |_| Ok(Value::Int(epoch_fields(epoch_now()).0)))));
    m.insert("month".into(), Value::Fun(FunValue::new_native("month", vec![], |_| Ok(Value::Int(epoch_fields(epoch_now()).1)))));
    m.insert("day".into(), Value::Fun(FunValue::new_native("day", vec![], |_| Ok(Value::Int(epoch_fields(epoch_now()).2)))));
    m.insert("hour".into(), Value::Fun(FunValue::new_native("hour", vec![], |_| Ok(Value::Int(epoch_fields(epoch_now()).3)))));
    m.insert("minute".into(), Value::Fun(FunValue::new_native("minute", vec![], |_| Ok(Value::Int(epoch_fields(epoch_now()).4)))));
    m.insert("second".into(), Value::Fun(FunValue::new_native("second", vec![], |_| Ok(Value::Int(epoch_fields(epoch_now()).5)))));
    m.insert("sleep".into(), Value::Fun(FunValue::new_native("sleep", vec!["ms".into()], |a| {
        let ms = match a.first() {
            Some(Value::Int(i)) => *i,
            _ => 0,
        };
        if ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(ms as u64));
        }
        Ok(Value::Void)
    })));
    Value::Record(m)
}
