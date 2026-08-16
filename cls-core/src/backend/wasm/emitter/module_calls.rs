//! module_calls.rs (Fase 1: extraido de cls-core/src/backend/wasm/emitter/calls.rs).

use super::*;

impl<'a> FuncEmitter<'a> {



    /// `.join(sep)` sobre una tupla: unroll estático (slots conocidos en compile-time).
    pub(crate) fn emit_tuple_join(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        let obj_ty = self
            .types
            .get(&expr_span(&member.object))
            .cloned()
            .unwrap_or(Type::Any);
        let slots = match &obj_ty {
            Type::Tuple(s) => s.clone(),
            _ => vec![],
        };
        self.emit_expression(&member.object)?;
        let ptr = self.fresh_local();
        self.body.push(Instruction::LocalSet(ptr));
        self.emit_expression(&c.args[0])?;
        let sep = self.fresh_local();
        self.body.push(Instruction::LocalSet(sep));
        let empty = self.intern_string("");
        self.emit_load_str(empty);
        let res = self.fresh_local();
        self.body.push(Instruction::LocalSet(res));
        for (i, slot) in slots.iter().enumerate() {
            if i > 0 {
                self.body.push(Instruction::LocalGet(res));
                self.body.push(Instruction::LocalGet(sep));
                self.host.call(HostFn::StrConcat, &mut self.body);
                self.body.push(Instruction::LocalSet(res));
            }
            let slot_ty = was_type(slot)?;
            let s_tmp = self.fresh_local();
            self.body.push(Instruction::LocalGet(ptr));
            self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            match slot_ty {
                WasTy::F64 => self.body.push(Instruction::F64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
                WasTy::I32 => self.body.push(Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                })),
                WasTy::I64 => self.body.push(Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                })),
            }
            match (slot_ty, slot) {
                (WasTy::F64, _) => self.host.call(HostFn::StrFloat, &mut self.body),
                (WasTy::I32, Type::Bool) => self.host.call(HostFn::StrBool, &mut self.body),
                (WasTy::I32, _) => self.host.call(HostFn::StrChar, &mut self.body),
                (WasTy::I64, Type::String) => {}
                (WasTy::I64, _) => self.host.call(HostFn::StrInt, &mut self.body),
            }
            self.body.push(Instruction::LocalSet(s_tmp));
            self.body.push(Instruction::LocalGet(res));
            self.body.push(Instruction::LocalGet(s_tmp));
            self.host.call(HostFn::StrConcat, &mut self.body);
            self.body.push(Instruction::LocalSet(res));
        }
        self.body.push(Instruction::LocalGet(res));
        Ok(())
    }



    /// `math.X(...)` -> host del módulo math.
    pub(crate) fn emit_math_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "abs" => {
                self.emit_expression(&c.args[0])?;
                match self.value_type(&c.args[0])? {
                    WasTy::F64 => self.host.call(FloatAbs, &mut self.body),
                    _ => self.host.call(IntAbs, &mut self.body),
                }
                Ok(())
            }
            "sqrt" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathSqrt, &mut self.body);
                Ok(())
            }
            "floor" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathFloor, &mut self.body);
                Ok(())
            }
            "ceil" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathCeil, &mut self.body);
                Ok(())
            }
            "round" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathRound, &mut self.body);
                Ok(())
            }
            "sin" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathSin, &mut self.body);
                Ok(())
            }
            "cos" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathCos, &mut self.body);
                Ok(())
            }
            "tan" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathTan, &mut self.body);
                Ok(())
            }
            "log" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.host.call(MathLog, &mut self.body);
                Ok(())
            }
            "pow" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.f64_promote(&c.args[1])?;
                self.host.call(MathPow, &mut self.body);
                Ok(())
            }
            "min" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.f64_promote(&c.args[1])?;
                self.host.call(MathMin, &mut self.body);
                Ok(())
            }
            "max" => {
                self.emit_expression(&c.args[0])?;
                self.f64_promote(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.f64_promote(&c.args[1])?;
                self.host.call(MathMax, &mut self.body);
                Ok(())
            }
            "random" => {
                self.host.call(MathRandom, &mut self.body);
                Ok(())
            }
            "range" => {
                self.emit_expression(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.host.call(MathRange, &mut self.body);
                Ok(())
            }
            _ => Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
    }



    /// `fs.X(...)` -> host del módulo fs (básico: exists/cwd/readFile/writeFile/listDir/mkdir/rm).
    pub(crate) fn emit_fs_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "exists" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsExists, &mut self.body);
                Ok(())
            }
            "cwd" => {
                self.host.call(FsCwd, &mut self.body);
                Ok(())
            }
            "readFile" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsReadFile, &mut self.body);
                Ok(())
            }
            "writeFile" => {
                self.emit_expression(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.host.call(FsWriteFile, &mut self.body);
                Ok(())
            }
            "listDir" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsListDir, &mut self.body);
                Ok(())
            }
            "mkdir" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsMkdir, &mut self.body);
                Ok(())
            }
            "rm" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(FsRm, &mut self.body);
                Ok(())
            }
            _ => Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
    }



    /// `http.X(...)` -> host del módulo http.
    pub(crate) fn emit_http_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "get" => {
                self.emit_expression(&c.args[0])?;
                self.host.call(HttpGet, &mut self.body);
                Ok(())
            }
            "post" => {
                self.emit_expression(&c.args[0])?;
                self.emit_expression(&c.args[1])?;
                self.host.call(HttpPost, &mut self.body);
                Ok(())
            }
            _ => Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
    }



    /// `os.X(...)` -> host del módulo os.
    pub(crate) fn emit_os_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "platform" => self.host.call(OsPlatform, &mut self.body),
            "arch" => self.host.call(OsArch, &mut self.body),
            "version" => self.host.call(OsVersion, &mut self.body),
            "hostname" => self.host.call(OsHostname, &mut self.body),
            "home" => self.host.call(OsHome, &mut self.body),
            "tempdir" => self.host.call(OsTempdir, &mut self.body),
            "cpus" => self.host.call(OsCpus, &mut self.body),
            "pid" => self.host.call(OsPid, &mut self.body),
            "uptime" => self.host.call(OsUptime, &mut self.body),
            "env" => {
                self.emit_expression(self.call_arg(c, 0, "os.env")?)?;
                self.host.call(OsEnv, &mut self.body);
            }
            "sep" => self.host.call(OsSep, &mut self.body),
            "isWindows" => self.host.call(OsIsWindows, &mut self.body),
            "isUnix" => self.host.call(OsIsUnix, &mut self.body),
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }



    /// `path.X(...)` -> host del módulo path.
    pub(crate) fn emit_path_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "join" => {
                self.emit_expression(self.call_arg(c, 0, "path.join")?)?;
                self.emit_expression(self.call_arg(c, 1, "path.join")?)?;
                self.host.call(PathJoin, &mut self.body);
            }
            "basename" => {
                self.emit_expression(self.call_arg(c, 0, "path.basename")?)?;
                self.host.call(PathBasename, &mut self.body);
            }
            "dirname" => {
                self.emit_expression(self.call_arg(c, 0, "path.dirname")?)?;
                self.host.call(PathDirname, &mut self.body);
            }
            "extname" => {
                self.emit_expression(self.call_arg(c, 0, "path.extname")?)?;
                self.host.call(PathExtname, &mut self.body);
            }
            "resolve" => {
                self.emit_expression(self.call_arg(c, 0, "path.resolve")?)?;
                self.host.call(PathResolve, &mut self.body);
            }
            "normalize" => {
                self.emit_expression(self.call_arg(c, 0, "path.normalize")?)?;
                self.host.call(PathNormalize, &mut self.body);
            }
            "isAbsolute" => {
                self.emit_expression(self.call_arg(c, 0, "path.isAbsolute")?)?;
                self.host.call(PathIsAbsolute, &mut self.body);
            }
            "sep" => self.host.call(PathSep, &mut self.body),
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }



    /// `process.X(...)` -> host del módulo process.
    pub(crate) fn emit_process_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "args" => self.host.call(ProcessArgs, &mut self.body),
            "cwd" => self.host.call(ProcessCwd, &mut self.body),
            "env" => {
                self.emit_expression(self.call_arg(c, 0, "process.env")?)?;
                self.host.call(ProcessEnv, &mut self.body);
            }
            "exit" => {
                self.emit_expression(self.call_arg(c, 0, "process.exit")?)?;
                self.host.call(ProcessExit, &mut self.body);
            }
            "pid" => self.host.call(ProcessPid, &mut self.body),
            "platform" => self.host.call(ProcessPlatform, &mut self.body),
            "title" => self.host.call(ProcessTitle, &mut self.body),
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }



    /// `time.X(...)` -> host del módulo time.
    pub(crate) fn emit_time_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "now" => self.host.call(TimeNow, &mut self.body),
            "seconds" => self.host.call(TimeSeconds, &mut self.body),
            "iso" => self.host.call(TimeIso, &mut self.body),
            "date" => self.host.call(TimeDate, &mut self.body),
            "clock" => self.host.call(TimeClock, &mut self.body),
            "year" => self.host.call(TimeYear, &mut self.body),
            "month" => self.host.call(TimeMonth, &mut self.body),
            "day" => self.host.call(TimeDay, &mut self.body),
            "hour" => self.host.call(TimeHour, &mut self.body),
            "minute" => self.host.call(TimeMinute, &mut self.body),
            "second" => self.host.call(TimeSecond, &mut self.body),
            "sleep" => {
                self.emit_expression(self.call_arg(c, 0, "time.sleep")?)?;
                self.host.call(TimeSleep, &mut self.body);
            }
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }



    /// `random.X(...)` -> host del módulo random.
    pub(crate) fn emit_random_call(&mut self, member: &MemberAccessExpr, c: &CallExpr) -> ClsResult<()> {
        use HostFn::*;
        match member.member.as_str() {
            "random" => self.host.call(RandomRandom, &mut self.body),
            "int" => {
                self.emit_expression(self.call_arg(c, 0, "random.int")?)?;
                self.emit_expression(self.call_arg(c, 1, "random.int")?)?;
                self.host.call(RandomInt, &mut self.body);
            }
            "float" => {
                let a0 = self.call_arg(c, 0, "random.float")?;
                let a1 = self.call_arg(c, 1, "random.float")?;
                self.emit_expression(a0)?;
                self.f64_promote(a0)?;
                self.emit_expression(a1)?;
                self.f64_promote(a1)?;
                self.host.call(RandomFloat, &mut self.body);
            }
            "uuid" => self.host.call(RandomUuid, &mut self.body),
            _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
        }
        Ok(())
    }



    /// Valida la aridad de una llamada a host de módulo y devuelve el arg `i`.
    /// Evita `c.args[i]` con índice fuera de rango (panic → error de compilación).
    pub(crate) fn call_arg<'e>(&self, c: &'e CallExpr, i: usize, fn_name: &str) -> ClsResult<&'e Expression> {
        c.args.get(i).ok_or_else(|| {
            crate::error::ClsError::compile_at(
                &format!("{} esperaba {} argumento(s), recibió {}", fn_name, i + 1, c.args.len()),
                &c.span,
            )
        })
    }



    /// Tipo de retorno de una llamada o miembro de un módulo stdlib.
    pub(crate) fn module_call_ret(&self, expr: &Expression) -> Option<WasTy> {
        if let Expression::Call(c) = expr {
            if let Expression::MemberAccess(member) = &*c.callee {
                if let Expression::Identifier(obj, _) = &*member.object {
                    if obj == "math" {
                        return match member.member.as_str() {
                            "sqrt" | "pow" | "min" | "max" | "floor" | "ceil" | "round"
                            | "random" | "sin" | "cos" | "tan" | "log" => Some(WasTy::F64),
                            "range" => Some(WasTy::I64),
                            // `abs` devuelve el tipo del primer argumento.
                            "abs" => {
                                let arg_ty = c.args.first()
                                    .and_then(|a| self.types.get(&expr_span(a)))
                                    .cloned()
                                    .unwrap_or(Type::Any);
                                if matches!(arg_ty, Type::Float | Type::F32 | Type::F64) {
                                    Some(WasTy::F64)
                                } else {
                                    Some(WasTy::I64)
                                }
                            }
                            _ => None,
                        };
                    }
                    if obj == "json" && member.member == "stringify" {
                        return Some(WasTy::I64);
                    }
                    if obj == "json" && member.member == "parse" {
                        return Some(WasTy::I64);
                    }
                    if obj == "fs" {
                        return match member.member.as_str() {
                            "exists" => Some(WasTy::I32),
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "http" {
                        return Some(WasTy::I64);
                    }
                    if obj == "os" {
                        return match member.member.as_str() {
                            "isWindows" | "isUnix" => Some(WasTy::I32),
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "path" {
                        return match member.member.as_str() {
                            "isAbsolute" => Some(WasTy::I32),
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "process" {
                        // exit es void: no reportar valor (rompería `print(exit(0))`).
                        return match member.member.as_str() {
                            "exit" => None,
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "time" {
                        // sleep es void: no reportar valor.
                        return match member.member.as_str() {
                            "sleep" => None,
                            _ => Some(WasTy::I64),
                        };
                    }
                    if obj == "random" {
                        return match member.member.as_str() {
                            "random" | "float" => Some(WasTy::F64),
                            "int" => Some(WasTy::I64),
                            _ => Some(WasTy::I64),
                        };
                    }
                }
            }
        }
        // Miembros de módulos sin llamada: math.PI / math.E
        if let Expression::MemberAccess(member) = expr {
            if let Expression::Identifier(obj, _) = &*member.object {
                if obj == "math" && (member.member == "PI" || member.member == "E") {
                    return Some(WasTy::F64);
                }
            }
        }
        None
    }

}