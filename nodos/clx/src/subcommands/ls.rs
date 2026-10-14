//! `clx ls` - Lista los paquetes locales (`modules/`) y globales (`~/.cls/modules`).

use crate::packages::{self, PackageInfo};

pub fn execute(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return 0;
    }
    let global_only = args.iter().any(|a| a == "-g" || a == "--global");
    let local_only = args.iter().any(|a| a == "-l" || a == "--local");
    let show_local = !global_only;
    let show_global = !local_only;

    let mut total = 0usize;

    if show_local {
        let dir = packages::local_modules_dir();
        let list = packages::local_packages();
        print_section(&format!("Paquetes locales ({})", dir.display()), &list);
        total += list.len();
    }
    if show_global {
        let dir = packages::global_modules_dir()
            .map(|d| d.display().to_string())
            .unwrap_or_else(|| "(sin HOME)".to_string());
        let list = packages::global_packages();
        print_section(&format!("Paquetes globales ({})", dir), &list);
        total += list.len();
    }

    if total == 0 {
        println!();
        println!("(no hay paquetes instalados)");
    }
    0
}

fn print_section(title: &str, list: &[PackageInfo]) {
    println!("{}:", title);
    if list.is_empty() {
        println!("  (ninguno)");
        return;
    }
    for p in list {
        let desc = if p.description.is_empty() {
            String::new()
        } else {
            format!("  {}", p.description)
        };
        let warn = if p.issues.is_empty() {
            String::new()
        } else {
            format!("  [!{} problema(s)]", p.issues.len())
        };
        println!(
            "  {:<28} [{}]  {}{}{}",
            p.full_name(),
            p.kind(),
            p.path.display(),
            desc,
            warn
        );
    }
    println!();
}

fn print_help() {
    println!("clx ls - Lista los paquetes instalados");
    println!();
    println!("Uso: clx ls [-l|--local] [-g|--global]");
    println!();
    println!("  -l, --local    Solo los paquetes locales (./modules)");
    println!("  -g, --global   Solo los paquetes globales (~/.cls/modules)");
}
