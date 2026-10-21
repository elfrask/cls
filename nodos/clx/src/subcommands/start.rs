use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn project_root() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut dir = Some(cwd.clone());
    while let Some(d) = dir {
        if d.join("cls.json").is_file() || d.join("modules").is_dir() {
            return d;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    cwd
}

fn clean_path(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{}", rest));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    p
}

fn add_bin(root: &Path, out: &mut Vec<PathBuf>) {
    if !root.is_dir() {
        return;
    }
    let direct = root.join("bin");
    if direct.is_dir() {
        if let Ok(p) = direct.canonicalize() {
            push_unique(out, clean_path(p));
        }
    }
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let bin = p.join("bin");
        if bin.is_dir() {
            if let Ok(c) = bin.canonicalize() {
                push_unique(out, clean_path(c));
            }
            continue;
        }
        // scope/{name}@{version}/bin
        if let Ok(children) = std::fs::read_dir(&p) {
            for c in children.flatten() {
                let cb = c.path().join("bin");
                if cb.is_dir() {
                    if let Ok(cc) = cb.canonicalize() {
                        push_unique(out, clean_path(cc));
                    }
                }
            }
        }
    }
}

fn push_unique(out: &mut Vec<PathBuf>, p: PathBuf) {
    if !out.iter().any(|x| x == &p) {
        out.push(p);
    }
}

fn collect_bins() -> Vec<PathBuf> {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = project_root();
    let mut bins: Vec<PathBuf> = Vec::new();

    // Proyecto: modules/ y bin/ del proyecto.
    add_bin(&root.join("modules"), &mut bins);
    let local_bin = root.join("bin");
    if local_bin.is_dir() {
        if let Ok(p) = local_bin.canonicalize() {
            push_unique(&mut bins, clean_path(p));
        }
    }

    // cls.json -> pathModules (ademas de modules/).
    let manifest_path = root.join("cls.json");
    if let Ok(m) = cls_core::config::ModuleManifest::from_file(&manifest_path) {
        for rel in m.path_modules {
            add_bin(&root.join(rel), &mut bins);
        }
    }
    // CWD modules (si se ejecuta desde un subdirectorio).
    if cwd != root {
        add_bin(&cwd.join("modules"), &mut bins);
    }

    // Globales: ~/.cls/modules y ~/.cls/bin.
    if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
        let h = PathBuf::from(home);
        add_bin(&h.join(".cls").join("modules"), &mut bins);
        let gb = h.join(".cls").join("bin");
        if gb.is_dir() {
            if let Ok(p) = gb.canonicalize() {
                push_unique(&mut bins, clean_path(p));
            }
        }
    }
    bins
}

fn resolve_command(name: &str, paths: &[PathBuf]) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() || name.contains('/') || name.contains('\\') {
        return if p.is_file() { Some(clean_path(p.to_path_buf())) } else { None };
    }
    let exts: Vec<String> = if cfg!(windows) {
        let raw = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        let mut v: Vec<String> = raw
            .split(';')
            .filter(|e| !e.is_empty())
            .map(|e| e.to_ascii_lowercase())
            .collect();
        v.push(String::new());
        v
    } else {
        vec![String::new()]
    };
    for dir in paths {
        for ext in &exts {
            let candidate = dir.join(format!("{}{}", name, ext));
            if !candidate.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = candidate.metadata() {
                    if meta.permissions().mode() & 0o111 == 0 {
                        continue;
                    }
                }
            }
            return Some(clean_path(candidate));
        }
    }
    None
}

fn run_command(args: &[String], paths: &[PathBuf]) -> i32 {
    let resolved = match resolve_command(&args[0], paths) {
        Some(p) => p,
        None => {
            eprintln!("clx start: comando no encontrado: {}", args[0]);
            return 127;
        }
    };
    let is_batch = cfg!(windows)
        && resolved
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"))
            .unwrap_or(false);
    let path_env = match env::join_paths(paths) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("clx start: PATH invalido: {}", e);
            return 1;
        }
    };
    let mut cmd = if is_batch {
        let shell = env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
        let mut c = Command::new(shell);
        c.arg("/C").arg(&resolved).args(&args[1..]);
        c
    } else {
        let mut c = Command::new(&resolved);
        c.args(&args[1..]);
        c
    };
    cmd.env("PATH", path_env);
    match cmd.status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("clx start: no se pudo lanzar '{}': {}", resolved.display(), e);
            1
        }
    }
}

pub fn execute(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("clx start - Abre una shell con el PATH de bins de los modulos");
        println!();
        println!("Uso: clx start [comando]");
        println!();
        println!("Sin comando abre una shell interactiva. Con comando lo ejecuta directamente");
        println!("(sin pasar por una shell) con modules/*/bin, pathModules, ~/.cls/modules y ~/.cls/bin.");
        return 0;
    }

    let bins = collect_bins();
    let mut paths: Vec<PathBuf> = bins.clone();
    if let Some(old) = env::var_os("PATH") {
        paths.extend(env::split_paths(&old));
    }

    for b in &bins {
        // eprintln!("[start] bin: {}", b.display());
    }

    if !args.is_empty() {
        return run_command(args, &paths);
    }

    let joined = match env::join_paths(&paths) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("clx start: PATH invalido: {}", e);
            return 1;
        }
    };
    let shell = if cfg!(windows) {
        env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into())
    } else {
        env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into())
    };
    let mut cmd = Command::new(shell);
    cmd.env("PATH", joined);
    match cmd.status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("clx start: no se pudo abrir la shell: {}", e);
            1
        }
    }
}
