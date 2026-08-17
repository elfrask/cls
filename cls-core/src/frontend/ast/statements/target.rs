//! AST - Target (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use serde::{Deserialize, Serialize};


/// Entorno de ejecución (SO, arquitectura, ABI, plataforma/HAL).
/// Para el binario portable se selecciona en runtime; para AOT embebido se fija
/// en build (`clx build --target <tripla>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub os: String,
    pub arch: String,
    pub abi: String,
    pub platform: String,
}


impl Target {
    /// Target del proceso actual (host del nodo). La arquitectura nativa de CLS
    /// es **`cls-arch`** (no la del hardware); `wasm` queda reservado.
    pub fn host() -> Self {
        let os = if cfg!(target_os = "windows") { "windows" }
        else if cfg!(target_os = "macos") { "macos" }
        else if cfg!(target_os = "linux") { "linux" }
        else { "none" };
        let abi = if cfg!(target_env = "msvc") { "msvc" }
        else if cfg!(target_env = "gnu") { "gnu" }
        else if cfg!(target_abi = "eabi") { "eabi" }
        else if cfg!(target_abi = "elfv1") { "elfv1" }
        else if cfg!(target_abi = "elfv2") { "elfv2" }
        else { "" };
        Self {
            os: os.to_string(),
            arch: "cls-arch".to_string(),
            abi: abi.to_string(),
            // `os` solo puede ser "windows" | "macos" | "linux" | "none", así que
            // el platform es "pc" para cualquier SO real (la rama "pc" era inalcanzable).
            platform: if os != "none" { "pc".to_string() } else { "none".to_string() },
        }
    }

    /// Parsea un target: tripla `arch-os-abi` (o `arch-vendor-os-abi`) o un
    /// nombre simple (SO conocido -> os; arch conocido -> arch).
    pub fn parse(s: &str) -> Self {
        if s == "cls-arch" {
            return Self {
                arch: "cls-arch".to_string(),
                os: String::new(),
                abi: String::new(),
                platform: "none".to_string(),
            };
        }
        let parts: Vec<&str> = s.split('-').collect();
        let (arch, os, abi) = match parts.as_slice() {
            [a, o] => (*a, *o, ""),
            [a, o, ab] => (*a, *o, *ab),
            [a, _vendor, o, ab] => (*a, *o, *ab),
            [one] => {
                const OSES: &[&str] = &["windows", "linux", "macos", "none", "bare-metal", "freebsd"];
                const ARCHES: &[&str] = &["cls-arch", "x86_64", "arm64", "aarch64", "arm", "riscv32", "riscv64", "avr"];
                if OSES.contains(one) {
                    ("", *one, "")
                } else if ARCHES.contains(one) {
                    (*one, "", "")
                } else {
                    (*one, "", "")
                }
            }
            _ => (s, "", ""),
        };
        Self {
            arch: arch.to_string(),
            os: os.to_string(),
            abi: abi.to_string(),
            platform: "none".to_string(),
        }
    }

    pub fn matches(&self, cond: &TargetCond) -> bool {
        match cond {
            TargetCond::Any => true,
            TargetCond::Os(s) => self.os == *s,
            TargetCond::Arch(s) => self.arch == *s,
            TargetCond::Abi(s) => self.abi == *s,
            TargetCond::Platform(s) => self.platform == *s,
            TargetCond::Target(s) => {
                let t = Target::parse(s);
                self.arch == t.arch
                    && self.os == t.os
                    && (t.abi.is_empty() || self.abi == t.abi)
            }
            TargetCond::Not(c) => !self.matches(c),
            TargetCond::And(a, b) => self.matches(a) && self.matches(b),
            TargetCond::Or(a, b) => self.matches(a) || self.matches(b),
        }
    }
}
