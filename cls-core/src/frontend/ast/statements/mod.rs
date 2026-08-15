//! AST â€” statements (Fase 1: extraido de frontend/ast.rs).

mod block;
mod case_clause;
mod case_pattern;
mod catch_clause;
mod class_decl;
mod class_member;
mod config_directive;
mod elif_branch;
mod enum_decl;
mod extension_decl;
mod extension_kind;
mod field_decl;
mod for_each_statement;
mod for_statement;
mod from_import_statement;
mod function_decl;
mod function_modifier;
mod if_statement;
mod import_name;
mod import_statement;
mod include_statement;
mod interface_decl;
mod interface_field;
mod meta_directive;
mod module_decl;
mod namespace_decl;
mod native_decl;
mod parameter;
mod signature_decl;
mod structure_decl;
mod switch_statement;
mod target;
mod target_cond;
mod try_statement;
mod type_alias_decl;
mod var_decl;
mod when_block;
mod when_branch;
mod while_statement;
mod with_statement;

pub use block::*;
pub use case_clause::*;
pub use case_pattern::*;
pub use catch_clause::*;
pub use class_decl::*;
pub use class_member::*;
pub use config_directive::*;
pub use elif_branch::*;
pub use enum_decl::*;
pub use extension_decl::*;
pub use extension_kind::*;
pub use field_decl::*;
pub use for_each_statement::*;
pub use for_statement::*;
pub use from_import_statement::*;
pub use function_decl::*;
pub use function_modifier::*;
pub use if_statement::*;
pub use import_name::*;
pub use import_statement::*;
pub use include_statement::*;
pub use interface_decl::*;
pub use interface_field::*;
pub use meta_directive::*;
pub use module_decl::*;
pub use namespace_decl::*;
pub use native_decl::*;
pub use parameter::*;
pub use signature_decl::*;
pub use structure_decl::*;
pub use switch_statement::*;
pub use target::*;
pub use target_cond::*;
pub use try_statement::*;
pub use type_alias_decl::*;
pub use var_decl::*;
pub use when_block::*;
pub use when_branch::*;
pub use while_statement::*;
pub use with_statement::*;

use super::cmx::*;
use super::expressions::*;
use crate::error::diagnostic::Span;
use serde::{Deserialize, Serialize};



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
    /// `Break(Span)` â€” el span permite ubicar el `break;` en errores.
    Break(Span),
    /// `Continue(Span)` â€” el span permite ubicar el `continue;` en errores.
    Continue(Span),

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

    // Nativo (FFI a librerÃ­as del sistema)
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
