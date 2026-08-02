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

    // Imports
    Import(ImportStatement),
    FromImport(FromImportStatement),
    Include(IncludeStatement),

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
    pub signatures: Vec<SignatureDecl>,
    pub span: Span,
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
    Record(Box<TypeAnnotation>, Box<TypeAnnotation>), // String{Integer}
    Fun(Vec<TypeAnnotation>, Box<TypeAnnotation>),     // (Int, String) -> Bool

    // Tipo nombrado (definido por usuario)
    Named(String, Vec<TypeAnnotation>), // Persona, Array<String>

    // Tipos acrónimos
    I32, I64, I16, I8, F32, F64, Cmx,
}

/// Visibilidad de miembros
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
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
            Statement::Import(i) => f.write_fmt(format_args!("import \"{}\"", i.path)),
            Statement::FromImport(fi) => f.write_fmt(format_args!("from \"{}\" import ...", fi.path)),
            Statement::Include(i) => f.write_fmt(format_args!("include \"{}\"", i.path)),
            Statement::Expression(e) => f.write_fmt(format_args!("expr: {:?}", e)),
            Statement::Config(c) => f.write_fmt(format_args!("#config({} = {})", c.key, c.value)),
            Statement::Cmx(c) => f.write_fmt(format_args!("<{}>", c.tag)),
            Statement::Meta(m) => f.write_fmt(format_args!("#{}", m.name)),
        }
    }
}
