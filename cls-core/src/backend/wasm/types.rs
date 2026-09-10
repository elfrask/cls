//! Tipos y conversiones CLS a WASM del backend (Fase 1: extraido de wasm/mod.rs).

use crate::error::ClsResult;
use crate::frontend::ast::*;
use crate::middleware::types::{LitVal, Type};
use wasm_encoder::{Instruction, MemArg, ValType};

/// Tipo WASM de un valor (los que dejan un único valor en el stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WasTy {
    I64,
    F64,
    I32,
}

impl WasTy {
    pub(super) fn val_type(self) -> ValType {
        match self {
            WasTy::I64 => ValType::I64,
            WasTy::F64 => ValType::F64,
            WasTy::I32 => ValType::I32,
        }
    }
}

/// Ancho de almacenamiento de un campo escalar de `structure` (ABI C): el valor
/// CLS viaja en registro como i64/f64/i32, pero en memoria ocupa 1/2/4/8 bytes
/// con alineación natural. Es lo que hace que un `structure` CLS sea binario-
/// compatible con el struct C equivalente (`sockaddr_in`, `WSADATA`, ...) al
/// pasarlo por FFI (ver docs/lenguaje/extension.md). `int`/`float` CLS son
/// i64/f64 (8 bytes); `Bool`/`Char` van como i32 (convención del FFI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FieldWidth {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

/// Campo de un `structure`: escalar (con acceso directo) o array inline `T[n]`
/// (solo ocupa espacio en el layout, como `char szDescription[257]` de WSADATA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FieldKind {
    Scalar(FieldWidth),
    Fixed(FieldWidth, i64),
}

impl FieldKind {
    pub(super) fn size(self) -> i64 {
        match self {
            FieldKind::Scalar(w) => w.size(),
            FieldKind::Fixed(w, n) => w.size() * n,
        }
    }

    pub(super) fn align(self) -> i64 {
        match self {
            FieldKind::Scalar(w) => w.align(),
            FieldKind::Fixed(w, _) => w.align(),
        }
    }

    /// Ancho de acceso escalar (`None` = array inline: solo layout).
    pub(super) fn scalar(self) -> Option<FieldWidth> {
        match self {
            FieldKind::Scalar(w) => Some(w),
            FieldKind::Fixed(..) => None,
        }
    }
}

impl FieldWidth {
    pub(super) fn size(self) -> i64 {
        match self {
            FieldWidth::I8 => 1,
            FieldWidth::I16 => 2,
            FieldWidth::I32 | FieldWidth::F32 => 4,
            FieldWidth::I64 | FieldWidth::F64 => 8,
        }
    }

    pub(super) fn align(self) -> i64 {
        self.size()
    }

    /// Carga el campo: `[addr:i32] -> valor CLS` (según el tipo de registro del
    /// campo, `value`). Los anchos angostos se extienden (sign-extend: todos los
    /// tipos enteros C de CLS son signed).
    pub(super) fn push_load(self, body: &mut Vec<Instruction<'static>>, value: WasTy) {
        body.push(match self {
            FieldWidth::I8 => Instruction::I32Load8S(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::I16 => Instruction::I32Load16S(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::I32 => Instruction::I32Load(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::I64 => Instruction::I64Load(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::F32 => Instruction::F32Load(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::F64 => Instruction::F64Load(MemArg { offset: 0, align: 0, memory_index: 0 }),
        });
        match self {
            FieldWidth::I8 | FieldWidth::I16 | FieldWidth::I32 => {
                if value == WasTy::I64 {
                    body.push(Instruction::I64ExtendI32S);
                }
            }
            FieldWidth::F32 => body.push(Instruction::F64PromoteF32),
            _ => {}
        }
    }

    /// Almacena el campo: `[addr:i32, valor CLS] -> []`. Convierte el valor de
    /// registro (`value`) al ancho crudo.
    pub(super) fn push_store(self, body: &mut Vec<Instruction<'static>>, value: WasTy) {
        match (value, self) {
            (WasTy::I64, FieldWidth::I8 | FieldWidth::I16 | FieldWidth::I32) => {
                body.push(Instruction::I32WrapI64)
            }
            (WasTy::I32, FieldWidth::I64) => body.push(Instruction::I64ExtendI32S),
            (WasTy::F64, FieldWidth::F32) => body.push(Instruction::F32DemoteF64),
            _ => {}
        }
        body.push(match self {
            FieldWidth::I8 => Instruction::I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::I16 => Instruction::I32Store16(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::I32 => Instruction::I32Store(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::I64 => Instruction::I64Store(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::F32 => Instruction::F32Store(MemArg { offset: 0, align: 0, memory_index: 0 }),
            FieldWidth::F64 => Instruction::F64Store(MemArg { offset: 0, align: 0, memory_index: 0 }),
        });
    }
}

/// Campo de `structure` a partir de su anotación. Los tipos crudos
/// (`i8`/`i16`/`i32`/`i64`/`f32`/`f64`) y los tipos C de `extension`
/// (`CChar`/`CShort`/`CInt`/`CLong`/`CDouble`/`CString`/punteros) fijan el ancho
/// C; el resto se deduce del tipo CLS (`int`->i64, `float`->f64,
/// `bool`->i32, objetos/punteros->i64). `T[n]` es un array inline (solo layout).
pub(super) fn field_width(ann: &TypeAnnotation, t: &Type) -> FieldKind {
    use crate::frontend::ast::TypeKind;
    if let TypeKind::Access(inner, TypeAccess::Index(n)) = &ann.kind {
        let elem = field_width(inner, &annotation_to_type(inner));
        let w = elem.scalar().unwrap_or(FieldWidth::I8);
        return FieldKind::Fixed(w, (*n).max(0) as i64);
    }
    let w = match &ann.kind {
        TypeKind::I8 => FieldWidth::I8,
        TypeKind::I16 => FieldWidth::I16,
        TypeKind::I32 => FieldWidth::I32,
        TypeKind::I64 => FieldWidth::I64,
        TypeKind::F32 => FieldWidth::F32,
        TypeKind::F64 => FieldWidth::F64,
        // Tipos C del FFI (mismos que la tabla de extension.md).
        TypeKind::Named(name, _) => match name.as_str() {
            "CChar" | "CUChar" => FieldWidth::I8,
            "CShort" | "CUShort" => FieldWidth::I16,
            "CInt" | "CUInt" | "CBool" => FieldWidth::I32,
            "CFloat" => FieldWidth::F32,
            "CDouble" => FieldWidth::F64,
            _ => return FieldKind::Scalar(cls_width(t)),
        },
        _ => return FieldKind::Scalar(cls_width(t)),
    };
    FieldKind::Scalar(w)
}

/// Ancho C por defecto de un campo según su tipo CLS.
fn cls_width(t: &Type) -> FieldWidth {
    match t {
        Type::Bool | Type::Char => FieldWidth::I32,
        Type::Float | Type::F32 | Type::F64 => FieldWidth::F64,
        _ => FieldWidth::I64,
    }
}
/// Convierte un Type CLS a su representación WASM.
pub(super) fn was_type(t: &Type) -> ClsResult<WasTy> {
    match t {
        Type::Int | Type::I8 | Type::I16 | Type::I32 | Type::I64 => Ok(WasTy::I64),
        Type::Float | Type::F32 | Type::F64 => Ok(WasTy::F64),
        Type::Bool => Ok(WasTy::I32),
        Type::Char => Ok(WasTy::I32),
        Type::String => Ok(WasTy::I64),
        Type::Cmx => Ok(WasTy::I64),
        Type::Callable => Ok(WasTy::I64),
        Type::Array(_) => Ok(WasTy::I64),
        Type::Tuple(_) => Ok(WasTy::I64),
        Type::Record(_, _) => Ok(WasTy::I64),
        Type::Shape(_) => Ok(WasTy::I64),
        Type::Literal(LitVal::Float(_)) => Ok(WasTy::F64),
        Type::Literal(LitVal::Bool(_)) => Ok(WasTy::I32),
        Type::Named(..) | Type::Literal(_) => Ok(WasTy::I64),
        Type::Fun(..) => Ok(WasTy::I64),
        Type::Any => Ok(WasTy::I64),
        Type::Json | Type::Value => Ok(WasTy::I64),
        Type::Union(members) => {
            let mut it = members.iter();
            let first = it.next().and_then(|m| was_type(m).ok());
            if let Some(f) = first {
                if it.all(|m| was_type(m).ok() == Some(f)) {
                    return Ok(f);
                }
            }
            Ok(WasTy::I64)
        }
        Type::Void | Type::Empty | Type::Null => Ok(WasTy::I64),
        other => Err(crate::error::ClsError::CompileError(format!(
            "Tipo '{}' no soportado por el backend WASM (subconjunto JIT)",
            other
        ))),
    }
}
/// Nombre de tipo builtin para `v is Tipo` (compile-time en el JIT).
pub(super) fn builtin_was_type(name: &str) -> Option<BuiltinTypeName> {    match name {
        "String" => Some(BuiltinTypeName::String),
        "Int" => Some(BuiltinTypeName::Int),
        "Float" => Some(BuiltinTypeName::Float),
        "Bool" => Some(BuiltinTypeName::Bool),
        "Char" => Some(BuiltinTypeName::Char),
        "Array" => Some(BuiltinTypeName::Array),
        "Tuple" => Some(BuiltinTypeName::Tuple),
        "Record" => Some(BuiltinTypeName::Record),
        "Cmx" | "cmx" | "CMX" => Some(BuiltinTypeName::Cmx),
        "Callable" => Some(BuiltinTypeName::Callable),
        "Null" => Some(BuiltinTypeName::Null),
        "Void" => Some(BuiltinTypeName::Void),
        "JSON" | "Json" | "json" => Some(BuiltinTypeName::Json),
        "Value" | "value" => Some(BuiltinTypeName::Value),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BuiltinTypeName {
    String,
    Int,
    Float,
    Bool,
    Char,
    Array,
    Tuple,
    Record,
    Cmx,
    Callable,
    Null,
    Void,
    Json,
    Value,
}

/// ¿El Type CLS del lado izquierdo coincide con el nombre builtin a la derecha?
pub(super) fn builtin_type_matches(t: &Type, name: &BuiltinTypeName) -> bool {
    match (name, t) {
        (BuiltinTypeName::String, Type::String) => true,
        (BuiltinTypeName::Int, Type::Int) => true,
        (BuiltinTypeName::Int, Type::I8 | Type::I16 | Type::I32 | Type::I64) => true,
        (BuiltinTypeName::Float, Type::Float | Type::F32 | Type::F64) => true,
        (BuiltinTypeName::Bool, Type::Bool) => true,
        (BuiltinTypeName::Char, Type::Char) => true,
        (BuiltinTypeName::Array, Type::Array(_)) => true,
        (BuiltinTypeName::Tuple, Type::Tuple(_)) => true,
        (BuiltinTypeName::Record, Type::Record(_, _)) => true,
        (BuiltinTypeName::Record, Type::Shape(_)) => true,
        (BuiltinTypeName::Cmx, Type::Cmx | Type::Value) => true,
        (BuiltinTypeName::Callable, Type::Callable | Type::Fun(_, _) | Type::Value) => true,
        (BuiltinTypeName::Null, Type::Null) => true,
        (BuiltinTypeName::Void, Type::Void | Type::Empty) => true,
        (BuiltinTypeName::Json, Type::Json | Type::Value | Type::Record(_, _) | Type::Array(_) | Type::Shape(_)) => true,
        (BuiltinTypeName::Value, _) => true,
        _ => false,
    }
}

pub(super) fn annotation_to_type(ann: &TypeAnnotation) -> Type {
    use crate::frontend::ast::TypeKind;
    match &ann.kind {
        TypeKind::Int | TypeKind::I32 | TypeKind::I64 | TypeKind::I16 | TypeKind::I8 => Type::Int,
        TypeKind::Float | TypeKind::F32 | TypeKind::F64 => Type::Float,
        TypeKind::String => Type::String,
        TypeKind::Bool => Type::Bool,
        TypeKind::Char => Type::Char,
        TypeKind::Any => Type::Any,
        TypeKind::Json => Type::Json,
        TypeKind::Value => Type::Value,
        TypeKind::Void | TypeKind::Empty => Type::Void,
        TypeKind::Array(inner) => Type::Array(Box::new(annotation_to_type(inner))),
        TypeKind::Tuple(items) => Type::Tuple(items.iter().map(annotation_to_type).collect()),
        TypeKind::Record(k, v) => {
            Type::Record(Box::new(annotation_to_type(k)), Box::new(annotation_to_type(v)))
        }
        // Shape literal `{ campo: tipo, ... }`.
        TypeKind::Shape(fields) => Type::Shape(
            fields
                .iter()
                .map(|(n, t)| (n.clone(), annotation_to_type(t)))
                .collect(),
        ),
        // Nombrados: los builtins genéricos (`Record<K,V>`, `Array<T>`, alias
        // básicos) se resuelven aquí (el typeck los resuelve en su propio
        // resolve_type_annotation; el emisor debe hacer lo mismo).
        TypeKind::Named(name, args) => match name.as_str() {
            "Record" | "Dict" | "Map" if args.len() == 2 => Type::Record(
                Box::new(annotation_to_type(&args[0])),
                Box::new(annotation_to_type(&args[1])),
            ),
            "Array" | "List" if args.len() == 1 => {
                Type::Array(Box::new(annotation_to_type(&args[0])))
            }
            "String" => Type::String,
            "Int" | "Integer" => Type::Int,
            "Float" | "Double" => Type::Float,
            "Bool" | "Boolean" => Type::Bool,
            "Char" => Type::Char,
            "Any" | "any" => Type::Any,
            "JSON" | "Json" | "json" => Type::Json,
            "Value" | "value" => Type::Value,
            // Struct(Nombre): struct de extensión -> se tipa como el struct
            // (member access por offsets del struct_defs).
            "Struct" if args.len() == 1 => {
                if let TypeKind::Named(sn, _) = &args[0].kind {
                    Type::Named(sn.clone(), vec![])
                } else {
                    Type::Named(name.clone(), args.iter().map(annotation_to_type).collect())
                }
            }
            _ => Type::Named(name.clone(), args.iter().map(annotation_to_type).collect()),
        },
        TypeKind::Cmx => Type::Cmx,
        _ => Type::Any,
    }
}
/// Código de tipo nativo para la firma de extensiones: i=int, f=float, b=bool,
/// c=char, s=string, v=void, I=int C de 32 bits (CInt/CChar/...: en WASM viaja
/// como i64; el host lo convierte a `NativeType::CInt` para el transmute i32).
/// El nombre del import codifica ret+params. Paridad con `native_type_from_ann`
/// del walker (cls-runtime/src/walker/interpreter.rs).
pub(super) fn ty_code(t: &Type) -> (char, WasTy) {
    match t {
        Type::String => ('s', WasTy::I64),
        Type::Float => ('f', WasTy::F64),
        Type::Bool => ('b', WasTy::I32),
        Type::Char => ('c', WasTy::I32),
        Type::Void => ('v', WasTy::I64),
        Type::Named(n, _) => match n.as_str() {
            // CDouble -> f64 (antes caía al default 'i': el import salía
            // `sqrt__ii@libm` y cranelift rechazaba el f64 del literal).
            "CDouble" | "Double" => ('f', WasTy::F64),
            // Enteros C de 32 bits: viajan en i64 por WASM; el host los tipa
            // como CInt (transmute a i32 real, paridad con el walker).
            "CInt" | "CUInt" | "CShort" | "CUShort" | "CChar" | "CUChar" => ('I', WasTy::I64),
            // CString: igual que String (i64 empaquetado ptr<<32|len); la letra
            // 's' hace que el wrapper lea el string del caller.
            "CString" => ('s', WasTy::I64),
            // CPtr / CLong / CULong: punteros y enteros nativos de 64 bits.
            "CPtr" | "CLong" | "CULong" => ('i', WasTy::I64),
            // CRecord / CArray / CStruct: puntero al layout de memoria lineal.
            // Letras propias ('r'/'a'/'S') para que el wrapper del JIT distinga
            // el marshalling (traduce offset wasm <-> dirección host; el ptr del
            // layout del WASM no es válido fuera del módulo). En el WASM viajan
            // como i64 (puntero), igual que CPtr.
            "CRecord" => ('r', WasTy::I64),
            "CArray" => ('a', WasTy::I64),
            "CStruct" => ('S', WasTy::I64),
            // CFloat (f32) no está soportado por el dispatcher: error claro al
            // registrar el host (native.rs da el error para args).
            "CFloat" => ('i', WasTy::I64),
            _ => ('i', WasTy::I64), // estructura o desconocido
        },
        _ => ('i', WasTy::I64),
    }
}

pub(super) fn code_to_was(c: char) -> WasTy {
    match c {
        'f' => WasTy::F64,
        'b' | 'c' => WasTy::I32,
        _ => WasTy::I64,
    }
}

pub(super) fn was_to_val(w: WasTy) -> ValType {
    match w {
        WasTy::F64 => ValType::F64,
        WasTy::I32 => ValType::I32,
        WasTy::I64 => ValType::I64,
    }
}
