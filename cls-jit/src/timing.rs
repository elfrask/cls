//! Timing del pipeline (CLS_JIT_TIMING=1 -> tiempos por fase a stderr).

use std::time::Instant;

/// `CLS_JIT_TIMING=1` -> imprime el tiempo de cada fase del pipeline a stderr.
pub fn jit_timing() -> bool {
    std::env::var("CLS_JIT_TIMING")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn tick(timing: bool, label: &str, start: Instant) -> Instant {
    if timing {
        eprintln!(
            "[JIT-TIMING] {:<26} {:>12.2} ms",
            label,
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
    Instant::now()
}
