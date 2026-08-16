//! binary.rs (Fase 1: extraido de cls-core/src/middleware/typeck/expressions.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_binary(&mut self, bin: &BinaryExpr) -> Type {
        use crate::frontend::token::Operator;

        // `is` con tipo builtin (`v is String`): el right es un nombre de tipo, no
        // una variable. Se registra el tipo del nombre en el span para el backend.
        let is_builtin_is = if bin.op == Operator::Is {
            match &*bin.right {
                Expression::Identifier(n, _) => builtin_type_name(n).is_some(),
                _ => false,
            }
        } else {
            false
        };

        let left = self.check_expression(&bin.left);
        let right = if is_builtin_is {
            if let Expression::Identifier(n, sp) = &*bin.right {
                let t = builtin_type_name(n).unwrap();
                self.types_by_span.insert(sp.clone(), t.clone());
                t
            } else {
                self.check_expression(&bin.right)
            }
        } else {
            self.check_expression(&bin.right)
        };

        match bin.op {
            Operator::Plus => {
                let is_str_l = matches!(left, Type::String);
                let is_str_r = matches!(right, Type::String);
                let is_num_l = matches!(left, Type::Int | Type::Float | Type::I32 | Type::I64);
                let is_num_r = matches!(right, Type::Int | Type::Float | Type::I32 | Type::I64);

                if is_str_l && is_str_r {
                    return Type::String;
                }
                if is_num_l && is_num_r {
                    if matches!(left, Type::Float) || matches!(right, Type::Float) {
                        return Type::Float;
                    }
                    return Type::Int;
                }
                // Int + Float -> Float
                if is_num_l && matches!(right, Type::Float) {
                    return Type::Float;
                }
                if matches!(left, Type::Float) && is_num_r {
                    return Type::Float;
                }
                // Magic method __add: clase con __add (left primero, luego right).
                // El operando se valida contra el parámetro del magic (M1: tipos
                // incompatibles producían basura de memoria).
                if self.named_magic_ret(&left, "__add").is_some() {
                    self.validate_magic_binary_operand(&left, &right, "__add", bin.span.clone());
                    return self.named_magic_ret(&left, "__add").unwrap();
                }
                if self.named_magic_ret(&right, "__add").is_some() {
                    self.validate_magic_binary_operand(&right, &left, "__add", bin.span.clone());
                    return self.named_magic_ret(&right, "__add").unwrap();
                }
                self.error(
                    &format!(
                        "Operador + no soportado entre {} y {} (en `{}`)",
                        left,
                        right,
                        format!(
                            "{} + {}",
                            expr_short_display(&bin.left),
                            expr_short_display(&bin.right)
                        )
                    ),
                    bin.span.clone(),
                )
            }
            Operator::Minus | Operator::Star | Operator::Slash | Operator::Percent | Operator::StarStar => {
                let l_ok = matches!(left, Type::Int | Type::Float | Type::I32 | Type::I64);
                let r_ok = matches!(right, Type::Int | Type::Float | Type::I32 | Type::I64);
                if l_ok && r_ok {
                    if matches!(left, Type::Float) || matches!(right, Type::Float) {
                        return Type::Float;
                    }
                    return Type::Int;
                }
                // Magic methods aritméticos: __sub/__mul/__div/__mod/__pow.
                let magic = match bin.op {
                    Operator::Minus => "__sub",
                    Operator::Star => "__mul",
                    Operator::Slash => "__div",
                    Operator::Percent => "__mod",
                    _ => "__pow",
                };
                // El operando se valida contra el parámetro del magic (M1).
                if self.named_magic_ret(&left, magic).is_some() {
                    self.validate_magic_binary_operand(&left, &right, magic, bin.span.clone());
                    return self.named_magic_ret(&left, magic).unwrap();
                }
                if self.named_magic_ret(&right, magic).is_some() {
                    self.validate_magic_binary_operand(&right, &left, magic, bin.span.clone());
                    return self.named_magic_ret(&right, magic).unwrap();
                }
                self.error(
                    &format!(
                        "Operador requiere tipos numéricos, encontró {} y {} (en `{} {} {}`)",
                        left,
                        right,
                        expr_short_display(&bin.left),
                        bin.op,
                        expr_short_display(&bin.right)
                    ),
                    bin.span.clone(),
                )
            }
            // Operadores bit a bit: exigen enteros y devuelven Int.
            Operator::Caret | Operator::ShiftLeft | Operator::ShiftRight => {
                let l_ok = matches!(left, Type::Int | Type::I32 | Type::I64 | Type::I8 | Type::I16);
                let r_ok = matches!(right, Type::Int | Type::I32 | Type::I64 | Type::I8 | Type::I16);
                if l_ok && r_ok {
                    return Type::Int;
                }
                self.error(
                    &format!("Operador bit a bit requiere enteros, encontró {} y {}", left, right),
                    bin.span.clone(),
                )
            }
            Operator::StrictEqual | Operator::NotEqual
            | Operator::LessThan | Operator::LessEqual
            | Operator::GreaterThan | Operator::GreaterEqual
            | Operator::In | Operator::Is => {
                // Validar los operandos del dispatch mágico (M1/M4): tipos
                // incompatibles producían basura de memoria o WASM inválido.
                match bin.op {
                    Operator::StrictEqual | Operator::NotEqual => {
                        if self.named_magic_ret(&left, "__equals").is_some() {
                            self.validate_magic_binary_operand(&left, &right, "__equals", bin.span.clone());
                        } else if self.named_magic_ret(&right, "__equals").is_some() {
                            self.validate_magic_binary_operand(&right, &left, "__equals", bin.span.clone());
                        }
                    }
                    Operator::LessThan | Operator::LessEqual
                    | Operator::GreaterThan | Operator::GreaterEqual => {
                        if self.named_magic_ret(&left, "__compare").is_some() {
                            self.validate_magic_binary_operand(&left, &right, "__compare", bin.span.clone());
                        } else if self.named_magic_ret(&right, "__compare").is_some() {
                            self.validate_magic_binary_operand(&right, &left, "__compare", bin.span.clone());
                        }
                    }
                    Operator::In => {
                        if self.named_magic_ret(&right, "__contains").is_some() {
                            self.validate_magic_binary_operand(&right, &left, "__contains", bin.span.clone());
                        }
                    }
                    _ => {}
                }
                Type::Bool
            }
            Operator::And | Operator::Or => {
                if !left.is_assignable_to(&Type::Bool) || !right.is_assignable_to(&Type::Bool) {
                    self.warn("Operador lógico requiere Bool", bin.span.clone());
                }
                Type::Bool
            }
            _ => Type::Any,
        }
    }



    pub(crate) fn check_unary(&mut self, un: &UnaryExpr) -> Type {
        let operand = self.check_expression(&un.operand);
        match un.op {
            UnaryOp::Negate => operand,
            UnaryOp::Not => Type::Bool,
            UnaryOp::BitwiseNot => Type::Int,
            UnaryOp::TypeOf => Type::String,
            UnaryOp::PostInc | UnaryOp::PreInc => Type::Int,
            UnaryOp::PostDec | UnaryOp::PreDec => Type::Int,
        }
    }

}