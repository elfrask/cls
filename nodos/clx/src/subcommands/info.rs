//! `clx info [ruta|paquete]` - Información de un proyecto o paquete CLS.

use std::path::Path;

use crate::packages::{self, PackageInfo, PackageSource};

pub fn execute(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return 0;
    }
    let target = args.iter().find(|a| !a.starts_with('-'));
    let result = match target {
        Some(t) => resolve_target(t),
        None => resolve_cwd(),
    };
    match result {
        Ok(info) => {
            print_info(&info);
            0
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    }
}

fn resolve_target(t: &str) -> Result<PackageInfo, String> {
    let p = Path::new(t);
    if p.exists() {
        if p.is_dir() {
            return packages::inspect_dir(p, PackageSource::Local)
                .ok_or_else(|| format!("'{}' no es un directorio de paquete válido", t));
        }
        let is_zip = p
            .extension()
            .map(|e| e.eq_ignore_ascii_case("zip") || e.eq_ignore_ascii_case("clsapp"))
            .unwrap_or(false);
        if is_zip {
            return packages::inspect_zip(p);
        }
        if p.file_name().map(|n| n == "cls.json").unwrap_or(false) {
            let dir = p.parent().unwrap_or(Path::new("."));
            return packages::inspect_dir(dir, PackageSource::Local)
                .ok_or_else(|| format!("'{}' no es un paquete válido", t));
        }
        return Err(format!("'{}' no es un directorio, cls.json ni .zip", t));
    }
    packages::find_by_name(t)
        .ok_or_else(|| format!("paquete '{}' no encontrado (ni en modules/ ni global)", t))
}

fn resolve_cwd() -> Result<PackageInfo, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    if !cwd.join("cls.json").exists() {
        return Err("no hay cls.json en el directorio actual (usa 'clx info <ruta|paquete>')".to_string());
    }
    packages::inspect_dir(&cwd, PackageSource::Local)
        .ok_or_else(|| "el directorio actual no es un paquete válido".to_string())
}

fn field(label: &str, value: &str) {
    if !value.is_empty() {
        println!("  {:<13}{}", format!("{}:", label), value);
    }
}

fn print_info(i: &PackageInfo) {
    println!("{}  [{}]  ({})", i.full_name(), i.kind(), i.source.label());
    field("descripción", &i.description);
    field("licencia", &i.license);
    if !i.authors.is_empty() {
        field("autores", &i.authors.join(", "));
    }
    field("target", &i.target);
    if i.is_module() {
        field("export.entry", i.export_entry.as_deref().unwrap_or(""));
        field("export.types", i.export_types.as_deref().unwrap_or(""));
    } else {
        field("entry", &i.entry);
    }
    if !i.dependencies.is_empty() {
        field("deps", &i.dependencies.join(", "));
    }
    field("ruta", &i.path.display().to_string());
    if i.issues.is_empty() {
        println!("  estado:       OK");
    } else {
        println!("  estado:       {} problema(s)", i.issues.len());
        for issue in &i.issues {
            println!("    - {}", issue);
        }
    }
}

fn print_help() {
    println!("clx info - Información de un paquete o proyecto CLS");
    println!();
    println!("Uso: clx info [ruta|paquete]");
    println!();
    println!("  <ruta>     directorio, cls.json o archivo .zip/.clsapp");
    println!("  <paquete>  nombre de un paquete local (modules/) o global (~/.cls/modules)");
    println!("  (sin arg)  el proyecto del directorio actual (cls.json)");
}
