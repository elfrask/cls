//! `clx audit` - Audita la integridad de los paquetes locales y globales.
//!
//! Comprueba el manifiesto, la existencia de `entry`/`export.entry`,
//! dependencias declaradas no instaladas y nombres duplicados.

use std::collections::HashMap;

use crate::packages::{self, PackageInfo};

pub fn execute(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return 0;
    }
    let global_only = args.iter().any(|a| a == "-g" || a == "--global");
    let local_only = args.iter().any(|a| a == "-l" || a == "--local");

    let mut scanned: Vec<PackageInfo> = Vec::new();
    if !global_only {
        scanned.extend(packages::local_packages());
    }
    if !local_only {
        scanned.extend(packages::global_packages());
    }

    if scanned.is_empty() {
        println!("No hay paquetes que auditar.");
        return 0;
    }

    // Detectar nombres duplicados (mismo `nombre@version` en más de un paquete).
    let mut seen: HashMap<String, Vec<&PackageInfo>> = HashMap::new();
    for p in &scanned {
        seen.entry(p.full_name()).or_default().push(p);
    }

    let mut total_issues = 0usize;
    println!("Auditando {} paquete(s):", scanned.len());
    println!();

    for p in &scanned {
        let mut issues: Vec<String> = p.issues.clone();

        // Dependencias declaradas no instaladas.
        for dep in &p.dependencies {
            if packages::find_by_name(dep).is_none() {
                issues.push(format!("dependencia '{}' no instalada", dep));
            }
        }
        // Nombres duplicados.
        if let Some(group) = seen.get(&p.full_name()) {
            if group.len() > 1 {
                issues.push(format!(
                    "'{}' está instalado {} veces ({} / {})",
                    p.full_name(),
                    group.len(),
                    group
                        .iter()
                        .map(|x| x.source.label())
                        .collect::<Vec<_>>()
                        .join(", "),
                    p.path.display()
                ));
            }
        }

        if issues.is_empty() {
            println!("  OK    {}  [{}]  {}", p.full_name(), p.kind(), p.path.display());
        } else {
            total_issues += issues.len();
            println!(
                "  FAIL  {}  [{}]  {}",
                p.full_name(),
                p.kind(),
                p.path.display()
            );
            for i in &issues {
                println!("          - {}", i);
            }
        }
    }

    println!();
    if total_issues == 0 {
        println!("Auditoría OK: no se encontraron problemas.");
        0
    } else {
        println!("Auditoría: {} problema(s) en {} paquete(s).", total_issues, scanned.len());
        1
    }
}

fn print_help() {
    println!("clx audit - Audita los paquetes instalados");
    println!();
    println!("Uso: clx audit [-l|--local] [-g|--global]");
    println!();
    println!("Verifica manifiesto, entry/export.entry, dependencias faltantes y duplicados.");
    println!("Devuelve código 1 si encuentra problemas.");
    println!();
    println!("  -l, --local    Solo paquetes locales (./modules)");
    println!("  -g, --global   Solo paquetes globales (~/.cls/modules)");
}
