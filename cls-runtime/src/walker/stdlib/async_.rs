use crate::walker::value::{FunValue, Value, Promise, Pollable, PollState};
use std::collections::HashMap;

/// Pollable que resuelve después de un delay (thread separado).
/// El poll espera (join) al thread y devuelve Ready cuando termina.
struct DelayTask {
    handle: Option<std::thread::JoinHandle<()>>,
    done: bool,
}

impl DelayTask {
    fn new(ms: u64) -> Self {
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        });
        Self { handle: Some(handle), done: false }
    }
}

impl Pollable for DelayTask {
    fn poll(&mut self, _interp: &mut crate::walker::interpreter::Interpreter) -> PollState {
        if self.done {
            return PollState::Ready(Value::Void);
        }
        if let Some(handle) = self.handle.take() {
            match handle.join() {
                Ok(_) => {
                    self.done = true;
                    PollState::Ready(Value::Void)
                }
                Err(_) => PollState::Rejected("delay thread panicked".to_string()),
            }
        } else {
            PollState::Pending
        }
    }
}

/// Pollable que resuelve cuando todas las promesas de la lista resuelven.
struct AllTask {
    promises: Vec<Promise>,
    index: usize,
    results: Vec<Value>,
}

impl AllTask {
    fn new(promises: Vec<Promise>) -> Self {
        let len = promises.len();
        Self { promises, index: 0, results: Vec::with_capacity(len) }
    }
}

impl Pollable for AllTask {
    fn poll(&mut self, interp: &mut crate::walker::interpreter::Interpreter) -> PollState {
        while self.index < self.promises.len() {
            let mut p = self.promises[self.index].clone();
            match p.poll(interp) {
                PollState::Ready(v) => {
                    self.results.push(v);
                    self.index += 1;
                }
                PollState::Rejected(e) => return PollState::Rejected(e),
                PollState::Pending => return PollState::Pending,
            }
        }
        PollState::Ready(Value::Array(self.results.clone()))
    }
}

/// Pollable que resuelve con la primera promesa que resuelve.
struct RaceTask {
    promises: Vec<Promise>,
    index: usize,
}

impl RaceTask {
    fn new(promises: Vec<Promise>) -> Self {
        Self { promises, index: 0 }
    }
}

impl Pollable for RaceTask {
    fn poll(&mut self, interp: &mut crate::walker::interpreter::Interpreter) -> PollState {
        while self.index < self.promises.len() {
            let mut p = self.promises[self.index].clone();
            match p.poll(interp) {
                PollState::Ready(v) => return PollState::Ready(v),
                PollState::Rejected(e) => return PollState::Rejected(e),
                PollState::Pending => {
                    self.index += 1;
                }
            }
        }
        PollState::Pending
    }
}

/// Devuelve el módulo `async`
pub fn module() -> Value {
    let mut m = HashMap::new();

    // async.delay(ms) -> Promise
    m.insert("delay".into(), Value::Fun(FunValue::new_native("delay", vec!["ms".into()], |a| {
        let ms = match a.first() { Some(Value::Int(i)) => *i as u64, _ => 0 };
        Ok(Value::Promise(Promise::new(Box::new(DelayTask::new(ms)))))
    })));

    // async.all(promises) -> Promise
    m.insert("all".into(), Value::Fun(FunValue::new_native("all", vec!["promises".into()], |a| {
        let promises = match a.first() {
            Some(Value::Array(arr)) => arr.iter().filter_map(|v| match v {
                Value::Promise(p) => Some(p.clone()),
                _ => None,
            }).collect(),
            _ => vec![],
        };
        Ok(Value::Promise(Promise::new(Box::new(AllTask::new(promises)))))
    })));

    // async.race(promises) -> Promise
    m.insert("race".into(), Value::Fun(FunValue::new_native("race", vec!["promises".into()], |a| {
        let promises = match a.first() {
            Some(Value::Array(arr)) => arr.iter().filter_map(|v| match v {
                Value::Promise(p) => Some(p.clone()),
                _ => None,
            }).collect(),
            _ => vec![],
        };
        Ok(Value::Promise(Promise::new(Box::new(RaceTask::new(promises)))))
    })));

    Value::Record(m)
}
