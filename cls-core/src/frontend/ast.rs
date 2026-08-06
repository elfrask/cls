use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Módulo/Archivo CLS completo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// Declaraciones/Statements del lenguaje
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    // Declaraciones de variables
    VarDecl(VarDecl),
    ConstDecl(VarDecl),

    // Funciones
    FunctionDecl(FunctionDecl),

    // Control de flujo
    If(IfStatement),
    While(WhileStatement),
    Loop(Block),
    For(ForStatement),
    ForEach(ForEachStatement),
    Switch(SwitchStatement),
    Try(TryStatement),
    With(WithStatement),
    Return(Option<Expression>),
    Break,
    Continue,

    // Clases y estructuras
    ClassDecl(ClassDecl),
    StructureDecl(StructureDecl),
    InterfaceDecl(InterfaceDecl),
    ModuleDecl(ModuleDecl),
    NamespaceDecl(NamespaceDecl),

    // Alias de tipos (compile-time)
    TypeAlias(TypeAliasDecl),

    // Enums (variantes constantes con identidad)
    EnumDecl(EnumDecl),

    // Imports
    Import(ImportStatement),
    FromImport(FromImportStatement),
    Include(IncludeStatement),

    // Nativo (FFI a librerías del sistema)
    Extension(ExtensionDecl),

    // Directiva multi-entorno (implementaciones por plataforma/arquitectura)
    When(WhenBlock),

    // Expresiones
    Expression(Expression),

    // Directivas
    Config(ConfigDirective),

    // CMX (JSX)
    Cmx(CmxElement),

    // Decoradores/Meta
    Meta(MetaDirective),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarDecl {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub value: Option<Expression>,
    pub visibility: Visibility,
    pub span: Span,
    /// Variable estática (miembro de clase static)
    #[serde(default)]
    pub is_static: bool,
    /// Variable de solo lectura: escritura solo interna (readonly)
    #[serde(default)]
    pub is_readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub visibility: Visibility,
    pub modifiers: Vec<FunctionModifier>,
    pub span: Span,
    /// Parámetros de tipo genérico `<T, U>` (compile-time)
    #[serde(default)]
    pub type_params: Vec<TypeParam>,
    /// Función nativa (sin cuerpo, declarada en `extension` o como símbolo del SO)
    #[serde(default)]
    pub is_native: bool,
}

/// Declaración nativa (`extension "lib" { ... }`) — símbolos de librerías del SO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionDecl {
    pub library: String,
    /// Tipo de extensión: `extension "lib" as <kind>` (default `C`).
    pub kind: ExtensionKind,
    pub declarations: Vec<NativeDecl>,
    pub span: Span,
}

/// Tipo de extensión (backend nativo). Enum fijo para los conocidos (rendimiento)
/// + `Custom` para tipos futuros sin tocar el core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionKind {
    C,
    Python,
    Wasm,
    Js,
    Wasi,
    Custom(String),
}

impl ExtensionKind {
    pub fn from_name(s: &str) -> Self {
        match s {
            "C" | "c" => ExtensionKind::C,
            "Python" | "python" => ExtensionKind::Python,
            "Wasm" | "wasm" => ExtensionKind::Wasm,
            "Js" | "js" | "JS" => ExtensionKind::Js,
            "Wasi" | "wasi" => ExtensionKind::Wasi,
            other => ExtensionKind::Custom(other.to_string()),
        }
    }

    pub fn name(&self) -> String {
        match self {
            ExtensionKind::C => "C".to_string(),
            ExtensionKind::Python => "Python".to_string(),
            ExtensionKind::Wasm => "Wasm".to_string(),
            ExtensionKind::Js => "Js".to_string(),
            ExtensionKind::Wasi => "Wasi".to_string(),
            ExtensionKind::Custom(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NativeDecl {
    Function(FunctionDecl),
    Structure(StructureDecl),
    Var(VarDecl),
}

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
        else if cfg!(target_abi = "elf") { "elf" }
        else { "" };
        Self {
            os: os.to_string(),
            arch: "cls-arch".to_string(),
            abi: abi.to_string(),
            platform: if os == "pc" || os != "none" { "pc".to_string() } else { "none".to_string() },
        }
    }

    /// Parsea un target: tripla `arch-os-abi` (o `arch-vendor-os-abi`) o un
    /// nombre simple (SO conocido → os; arch conocido → arch).
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

/// Condición de la directiva `when` (selección por entorno).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetCond {
    Any,
    Os(String),
    Arch(String),
    Abi(String),
    Platform(String),
    Target(String),
    Not(Box<TargetCond>),
    And(Box<TargetCond>, Box<TargetCond>),
    Or(Box<TargetCond>, Box<TargetCond>),
}

/// Directiva multi-entorno: todas las ramas se compilan; en runtime (o en build
/// para AOT) se selecciona la que coincide con el target del entorno actual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhenBlock {
    pub branches: Vec<WhenBranch>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhenBranch {
    pub cond: TargetCond,
    pub block: Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub default_value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionModifier {
    Async,
    Sync,
    Static,
    Export,
    Private,
    Public,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_block: Block,
    pub elif_branches: Vec<ElifBranch>,
    pub else_block: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElifBranch {
    pub condition: Expression,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhileStatement {
    pub condition: Expression,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForStatement {
    pub init: Option<Box<Statement>>,
    pub condition: Option<Expression>,
    pub update: Option<Expression>,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForEachStatement {
    pub item_name: String,
    pub index_name: Option<String>,
    pub iterable: Expression,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchStatement {
    pub value: Expression,
    pub cases: Vec<CaseClause>,
    pub default: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseClause {
    pub pattern: CasePattern,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CasePattern {
    Literal(Literal),
    Identifier(String),
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TryStatement {
    pub try_block: Block,
    pub catch_clauses: Vec<CatchClause>,
    pub finally_block: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchClause {
    pub param_name: String,
    pub param_type: Option<TypeAnnotation>,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithStatement {
    pub name: String,
    pub value: Expression,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDecl {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub body: Vec<ClassMember>,
    pub span: Span,
    /// Parámetros de tipo genérico `<T>` (compile-time)
    #[serde(default)]
    pub type_params: Vec<TypeParam>,
    /// Visibilidad (export → disponible en módulos importados)
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassMember {
    Method(FunctionDecl),
    Property(VarDecl),
    Constructor(FunctionDecl),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
    /// Visibilidad (export → disponible en módulos importados)
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub default_value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub fields: Vec<InterfaceField>,
    pub signatures: Vec<SignatureDecl>,
    pub span: Span,
}

/// Campo tipado de una interface (shape): `nombre: tipo`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceField {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

/// Alias de tipo (compile-time): `alias Vec3 = (Int, Int, Int);`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAliasDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

/// Declaración de enum: `enum Color { Rojo, Verde, Azul };`
/// Las variantes son constantes con identidad única (índice dentro del enum).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub span: Span,
    /// Visibilidad (export → disponible en módulos importados)
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDecl {
    pub name: String,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceDecl {
    pub name: String,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatement {
    pub path: String,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FromImportStatement {
    pub path: String,
    pub names: Vec<ImportName>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportName {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeStatement {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDirective {
    pub key: String,
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaDirective {
    pub name: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

/// Bloque de código { ... }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// Expresiones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expression {
    // Literales
    Literal(Literal),

    // Identificadores
    Identifier(String, Span),

    // Operaciones binarias
    Binary(BinaryExpr),

    // Operaciones unarias
    Unary(UnaryExpr),

    // Llamadas
    Call(CallExpr),

    // Acceso a miembro
    MemberAccess(MemberAccessExpr),

    // Indexado
    Index(IndexExpr),

    // Arrays
    Array(ArrayExpr),

    // Tuplas (arrays inmutables)
    Tuple(TupleExpr),

    // Records/Objects
    Record(RecordExpr),

    // Funciones flecha
    ArrowFunction(ArrowFunctionExpr),

    // If como expresión
    Conditional(ConditionalExpr),

    // Asignación
    Assignment(AssignmentExpr),

    // CMX (JSX)
    Cmx(CmxElement),

    // Paréntesis
    Parenthesized(Box<Expression>, Span),

    // Interpolación de strings
    StringInterpolation(StringInterpolation),

    // Namespace access
    NamespaceAccess(String, String, Span), // name::identifier

    // Await: espera una expresion
    Await(Box<Expression>, Span),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Literal {
    pub kind: LiteralKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralKind {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Char(char),
    Null,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryExpr {
    pub left: Box<Expression>,
    pub op: crate::frontend::token::Operator,
    pub right: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Negate,     // -x
    Not,        // !x
    BitwiseNot, // ~x
    TypeOf,     // typeof x
    PostInc,    // x++
    PostDec,    // x--
    PreInc,     // ++x
    PreDec,     // --x
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallExpr {
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberAccessExpr {
    pub object: Box<Expression>,
    pub member: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexExpr {
    pub object: Box<Expression>,
    pub index: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrayExpr {
    pub elements: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TupleExpr {
    pub elements: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordExpr {
    pub entries: Vec<(String, Expression)>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowFunctionExpr {
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Box<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalExpr {
    pub condition: Box<Expression>,
    pub then_expr: Box<Expression>,
    pub else_expr: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentExpr {
    pub target: Box<Expression>,
    pub op: crate::frontend::token::Operator,
    pub value: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringInterpolation {
    pub parts: Vec<InterpolationPart>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterpolationPart {
    Text(String),
    Expr(Expression),
}

/// CMX Element (JSX nativo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmxElement {
    pub tag: String,
    pub attributes: Vec<CmxAttribute>,
    pub children: Vec<CmxChild>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmxAttribute {
    pub name: String,
    pub value: Option<CmxAttributeValue>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CmxAttributeValue {
    String(String),
    Expression(Box<Expression>),
    Shorthand(String), // {value} → value={value}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CmxChild {
    Text(String),
    Expression(Box<Expression>),
    Element(Box<CmxElement>),
}

/// Anotación de tipo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAnnotation {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    // Tipos primitivos
    Int,
    Float,
    String,
    Bool,
    Char,
    Any,
    Unknown,
    Null,
    Void,
    Empty,

    // Tipos con parámetros
    Array(Box<TypeAnnotation>),
    Tuple(Vec<TypeAnnotation>),        // (Int, String) heterogéneo
    Union(Vec<TypeAnnotation>),        // "a" | "b" | 5
    Record(Box<TypeAnnotation>, Box<TypeAnnotation>), // String{Integer}
    Fun(Vec<TypeAnnotation>, Box<TypeAnnotation>),     // (Int, String) -> Bool
    Literal(LiteralKind),              // "d", 5, true (literal type)
    Access(Box<TypeAnnotation>, TypeAccess), // T["key"] | T[0]
    Phantom(Box<TypeAnnotation>),      // !T — param que no participa en el tipo

    // Tipo nombrado (definido por usuario)
    Named(String, Vec<TypeAnnotation>), // Persona, Array<String>

    // Tipos acrónimos
    I32, I64, I16, I8, F32, F64, Cmx,
}

/// Acceso a un miembro/posición de un tipo (compile-time)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeAccess {
    Key(String),  // T["field"]
    Index(usize), // T[0]
}

/// Parámetro de tipo genérico (con default opcional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeParam {
    pub name: String,
    pub default: Option<TypeAnnotation>,
    pub span: Span,
}

/// Visibilidad de miembros
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Export,
    Default,
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::VarDecl(v) => f.write_fmt(format_args!("var {} = ...", v.name)),
            Statement::ConstDecl(v) => f.write_fmt(format_args!("const {} = ...", v.name)),
            Statement::FunctionDecl(fd) => f.write_fmt(format_args!("function {}(...) -> ...", fd.name)),
            Statement::If(_) => f.write_str("if (...)"),
            Statement::While(_) => f.write_str("while (...)"),
            Statement::Loop(_) => f.write_str("loop"),
            Statement::For(_) => f.write_str("for (...)"),
            Statement::ForEach(_) => f.write_str("for each ..."),
            Statement::Switch(_) => f.write_str("switch (...)"),
            Statement::Try(_) => f.write_str("try"),
            Statement::With(_) => f.write_str("with ..."),
            Statement::Return(_) => f.write_str("return"),
            Statement::Break => f.write_str("break"),
            Statement::Continue => f.write_str("continue"),
            Statement::ClassDecl(c) => f.write_fmt(format_args!("class {}", c.name)),
            Statement::StructureDecl(s) => f.write_fmt(format_args!("structure {}", s.name)),
            Statement::InterfaceDecl(i) => f.write_fmt(format_args!("interface {}", i.name)),
            Statement::ModuleDecl(m) => f.write_fmt(format_args!("module {}", m.name)),
            Statement::NamespaceDecl(n) => f.write_fmt(format_args!("namespace {}", n.name)),
            Statement::TypeAlias(t) => f.write_fmt(format_args!("alias {} = ...", t.name)),
            Statement::EnumDecl(e) => f.write_fmt(format_args!("enum {}", e.name)),
            Statement::Import(i) => f.write_fmt(format_args!("import \"{}\"", i.path)),
            Statement::FromImport(fi) => f.write_fmt(format_args!("from \"{}\" import ...", fi.path)),
            Statement::Include(i) => f.write_fmt(format_args!("include \"{}\"", i.path)),
            Statement::Extension(e) => f.write_fmt(format_args!("extension \"{}\" as {}", e.library, e.kind.name())),
            Statement::When(w) => f.write_fmt(format_args!("when {{ {} rama(s) }}", w.branches.len())),
            Statement::Expression(e) => f.write_fmt(format_args!("expr: {:?}", e)),
            Statement::Config(c) => f.write_fmt(format_args!("#config({} = {})", c.key, c.value)),
            Statement::Cmx(c) => f.write_fmt(format_args!("<{}>", c.tag)),
            Statement::Meta(m) => f.write_fmt(format_args!("#{}", m.name)),
        }
    }
}
