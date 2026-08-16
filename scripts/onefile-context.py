#!/usr/bin/env python3
"""onefile-context.py - Genera un markdown con todo el proyecto para enviar a una IA.

Uso:
  python scripts/onefile-context.py [--out archivo.md] [--include-dir dir] [--exclude pattern]

Por defecto genera 'contexto-completo.md' en la raíz del proyecto.
Los archivos en .gitignore se excluyen automáticamente.
"""

import os
import fnmatch
import mimetypes
import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Extensiones de archivos de texto (incluir contenido)
TEXT_EXTENSIONS = {
    # Rust
    ".rs", ".toml", ".lock",
    # Documentacion
    ".md", ".txt", ".json", ".yaml", ".yml",
    # Config
    ".cfg", ".ini", ".gitignore",
    # CLS
    ".clsx", ".clsapp",
    # Scripts
    ".py", ".cmd", ".sh", ".ps1",
    # Web
    ".html", ".css", ".js", ".ts", ".tsx",
    # Otros
    ".xml", ".sql", ".env",
}

# Archivos a ignorar (nombres exactos)
IGNORE_FILES = {
    "Cargo.lock", ".gitignore",
    "out.txt", "err.txt",
    "contexto-completo.md",  # el propio output
}

# Directorios a ignorar completamente
IGNORE_DIRS = {
    "target", ".git", "__pycache__",
    "node_modules", ".vscode",
}


def load_gitignore(path: Path) -> list[str]:
    """Carga patrones .gitignore de un directorio."""
    patterns = []
    gi = path / ".gitignore"
    if gi.exists():
        with open(gi, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#"):
                    patterns.append(line)
    return patterns


def is_text_file(path: Path) -> bool:
    """Determina si un archivo es de texto basado en su extensión."""
    ext = path.suffix.lower()
    if ext in TEXT_EXTENSIONS:
        return True
    # Intentar detectar por MIME
    mime, _ = mimetypes.guess_type(str(path))
    if mime and mime.startswith("text/"):
        return True
    return False


def should_ignore(name: str, patterns: list[str]) -> bool:
    """Verifica si un nombre/patrón debe ser ignorado."""
    for pattern in patterns:
        if fnmatch.fnmatch(name, pattern):
            return True
        # Patron de directorio como "target/"
        if fnmatch.fnmatch(name, pattern.rstrip("/")):
            return True
    return False


def walk_project(root: Path, exclude_patterns: list[str]) -> list[Path]:
    """Recorre el proyecto respetando .gitignore y exclusiones."""
    files = []
    gitignore_patterns = load_gitignore(root)
    all_patterns = gitignore_patterns + exclude_patterns

    for dirpath, dirnames, filenames in os.walk(root):
        dirpath_p = Path(dirpath)

        # Saltar directorios ignorados
        rel_dir = dirpath_p.relative_to(root).as_posix()
        if rel_dir == ".":
            rel_dir = ""

        # Filtrar directorios
        dirnames[:] = [
            d for d in dirnames
            if d not in IGNORE_DIRS
            and not should_ignore(d, all_patterns)
            and not should_ignore(f"{rel_dir}/{d}" if rel_dir else d, all_patterns)
        ]

        # Saltar directorios completos que son ignorados
        if any(part in IGNORE_DIRS for part in dirpath_p.parts):
            continue

        # Recoger archivos
        for fname in sorted(filenames):
            if fname in IGNORE_FILES:
                continue

            rel_path = f"{rel_dir}/{fname}" if rel_dir else fname

            if should_ignore(rel_path, all_patterns) or should_ignore(fname, all_patterns):
                continue

            fpath = dirpath_p / fname
            if is_text_file(fpath):
                files.append(fpath)

    return files


def read_file_safe(path: Path) -> str:
    """Lee un archivo con manejo de errores."""
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            return f.read()
    except Exception as e:
        return f"[Error al leer: {e}]"


def generate_context(output: str, exclude: list[str]):
    """Genera el archivo markdown con el contexto completo."""
    root = ROOT

    print(f"Escaneando: {root}")
    files = walk_project(root, exclude)
    print(f"Archivos encontrados: {len(files)}")

    # Ordenar: primero directorios importantes
    def sort_key(p: Path):
        rel = p.relative_to(root).as_posix()
        # Priorizar archivos por directorio
        priority = 0
        if rel.startswith("agent-context/"):
            priority = 1
        elif rel.startswith("cls-core/"):
            priority = 2
        elif rel.startswith("cls-runtime/"):
            priority = 3
        elif rel.startswith("nodos/"):
            priority = 4
        elif rel.startswith("docs/"):
            priority = 5
        elif rel.startswith("examples/"):
            priority = 6
        elif rel.startswith("scripts/"):
            priority = 7
        return (priority, rel)

    files.sort(key=sort_key)

    outpath = root / output
    print(f"Generando: {outpath}")

    with open(outpath, "w", encoding="utf-8") as out:
        out.write(f"# Contexto completo del proyecto CLS 2.0\n\n")
        out.write(f"Generado: {__file__}\n\n")
        out.write(f"Total de archivos: {len(files)}\n\n")
        out.write("---\n\n")

        for fpath in files:
            rel = fpath.relative_to(root).as_posix()
            content = read_file_safe(fpath)
            lines = content.count("\n") + 1

            out.write(f"## `{rel}` ({lines} líneas)\n\n")
            out.write("```\n")
            out.write(content)
            if not content.endswith("\n"):
                out.write("\n")
            out.write("```\n\n")

    total_chars = outpath.stat().st_size
    print(f"[OK] Generado: {outpath} ({total_chars:,} bytes, {len(files)} archivos)")


def main():
    parser = argparse.ArgumentParser(
        description="Genera un markdown con todo el proyecto para IA"
    )
    parser.add_argument(
        "--out",
        default="contexto-completo.md",
        help="Archivo de salida (default: contexto-completo.md)",
    )
    parser.add_argument(
        "--exclude",
        action="append",
        default=[],
        help="Patrón adicional a excluir (ej: *.rs, docs/*)",
    )
    args = parser.parse_args()

    generate_context(args.out, args.exclude)


if __name__ == "__main__":
    main()
