//! Calls: emit_call, host/module calls, math/fs/http/os/path/process/time/random (Fase 1: extraido de emitter/mod.rs).

use super::*;

impl<'a> FuncEmitter<'a> {


    /// `.join(sep)` sobre una tupla: unroll estÃƒÂ¡tico (slots conocidos en compile-time).
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


    /// `math.X(...)` Ã¢â€ â€™ host del mÃƒÂ³dulo math.
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


    /// `fs.X(...)` Ã¢â€ â€™ host del mÃƒÂ³dulo fs (bÃƒÂ¡sico: exists/cwd/readFile/writeFile/listDir/mkdir/rm).
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


    /// `http.X(...)` Ã¢â€ â€™ host del mÃƒÂ³dulo http.
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


    /// `os.X(...)` Ã¢â€ â€™ host del mÃƒÂ³dulo os.
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


    /// `path.X(...)` Ã¢â€ â€™ host del mÃƒÂ³dulo path.
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


    /// `process.X(...)` Ã¢â€ â€™ host del mÃƒÂ³dulo process.
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


    /// `time.X(...)` Ã¢â€ â€™ host del mÃƒÂ³dulo time.
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


    /// `random.X(...)` Ã¢â€ â€™ host del mÃƒÂ³dulo random.
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


    /// Valida la aridad de una llamada a host de mÃƒÂ³dulo y devuelve el arg `i`.
    /// Evita `c.args[i]` con ÃƒÂ­ndice fuera de rango (panic Ã¢â€ â€™ error de compilaciÃƒÂ³n).
    pub(crate) fn call_arg<'e>(&self, c: &'e CallExpr, i: usize, fn_name: &str) -> ClsResult<&'e Expression> {
        c.args.get(i).ok_or_else(|| {
            crate::error::ClsError::compile_at(
                &format!("{} esperaba {} argumento(s), recibiÃƒÂ³ {}", fn_name, i + 1, c.args.len()),
                &c.span,
            )
        })
    }


    /// Tipo de retorno de una llamada o miembro de un mÃƒÂ³dulo stdlib.
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
                        // exit es void: no reportar valor (romperÃƒÂ­a `print(exit(0))`).
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
        // Miembros de mÃƒÂ³dulos sin llamada: math.PI / math.E
        if let Expression::MemberAccess(member) = expr {
            if let Expression::Identifier(obj, _) = &*member.object {
                if obj == "math" && (member.member == "PI" || member.member == "E") {
                    return Some(WasTy::F64);
                }
            }
        }
        None
    }


    pub(crate) fn emit_call(&mut self, c: &CallExpr) -> ClsResult<()> {
        // Constructor de structure: `Punto(3, 4)` Ã¢â€ â€™ alloc + stores.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(info) = self.struct_defs.get(name).cloned() {
                self.body.push(Instruction::I64Const(info.total));
                let alloc = self.func_indexes["__alloc"];
                self.body.push(Instruction::Call(alloc));
                let ptr = self.fresh_local();
                self.body.push(Instruction::LocalSet(ptr));
                self.body.push(Instruction::LocalGet(ptr));
                self.body.push(Instruction::I64Const(info.def_id as i64));
                self.emit_i64_store(0);
                self.body.push(Instruction::LocalGet(ptr));
                self.body
                    .push(Instruction::I64Const(info.fields.len() as i64));
                self.emit_i64_store(8);
                for (i, (_, _, w)) in info.fields.iter().enumerate() {
                    if i < c.args.len() {
                        self.emit_expression(&c.args[i])?;
                    } else {
                        self.body.push(Instruction::I64Const(0));
                    }
                    let val_tmp = self.fresh_local_ty(*w);
                    let addr_tmp = self.fresh_local();
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalSet(val_tmp),
                        WasTy::I32 => Instruction::LocalSet(val_tmp),
                        WasTy::I64 => Instruction::LocalSet(val_tmp),
                    });
                    self.body.push(Instruction::LocalGet(ptr));
                    self.body.push(Instruction::I64Const(info.offsets[i]));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::LocalSet(addr_tmp));
                    self.body.push(Instruction::LocalGet(addr_tmp));
                    self.body.push(Instruction::I32WrapI64);
                    self.body.push(match w {
                        WasTy::F64 => Instruction::LocalGet(val_tmp),
                        WasTy::I32 => Instruction::LocalGet(val_tmp),
                        WasTy::I64 => Instruction::LocalGet(val_tmp),
                    });
                    match w {
                        WasTy::F64 => self.body.push(Instruction::F64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        })),
                        WasTy::I32 => self.body.push(Instruction::I32Store(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        })),
                        WasTy::I64 => self.body.push(Instruction::I64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        })),
                    }
                }
                self.body.push(Instruction::LocalGet(ptr));
                return Ok(());
            }
        }
        // Constructor de clase: `Clase(args)` Ã¢â€ â€™ alloc + vtable + init fields + ctor.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(info) = self.class_defs.get(name).cloned() {
                self.body.push(Instruction::I64Const(info.total));
                let alloc = self.func_indexes["__alloc"];
                self.body.push(Instruction::Call(alloc));
                let obj = self.fresh_local();
                self.body.push(Instruction::LocalSet(obj));
                // vtable_ptr[0] = vtable_start, class_id[8] = id
                self.body.push(Instruction::LocalGet(obj));
                self.body
                    .push(Instruction::I64Const(info.vtable_start as i64));
                self.emit_i64_store(0);
                self.body.push(Instruction::LocalGet(obj));
                self.body.push(Instruction::I64Const(info.class_id as i64));
                self.emit_i64_store(8);
                // init fields a 0
                for (_fn, _t, w, off, _vis) in &info.fields {
                    self.body.push(Instruction::LocalGet(obj));
                    self.body.push(Instruction::I64Const(*off));
                    self.body.push(Instruction::I64Add);
                    self.body.push(Instruction::I32WrapI64);
                    match w {
                        WasTy::F64 => self
                            .body
                            .push(Instruction::F64Const(Ieee64::new(0.0f64.to_bits()))),
                        WasTy::I32 => self.body.push(Instruction::I32Const(0)),
                        WasTy::I64 => self.body.push(Instruction::I64Const(0)),
                    }
                    match w {
                        WasTy::F64 => self.body.push(Instruction::F64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        })),
                        WasTy::I32 => self.body.push(Instruction::I32Store(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        })),
                        WasTy::I64 => self.body.push(Instruction::I64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        })),
                    }
                }
                // call Clase::__ctor (o el del padre si no se define) con me.
                // Solo se pushea `me`+args si EXISTE el ctor: si la clase no lo
                // define, el stack debe quedar limpio (el leftover rompÃƒÂ­a la
                // validaciÃƒÂ³n WASM en `__init_globals`, que no tiene resultado
                // que lo consuma Ã¢â‚¬â€ el modo archivo lo enmascaraba con `return`).
                let callsite = c.span.clone();
                let mut cur = Some(name.to_string());
                while let Some(cls) = cur {
                    if let Some(idx) = self.func_indexes.get(&format!("{}::__ctor", cls)) {
                        self.body.push(Instruction::LocalGet(obj));
                        for a in &c.args {
                            self.emit_expression(a)?;
                        }
                        self.emit_call_site(&callsite);
                        self.body.push(Instruction::Call(*idx));
                        break;
                    }
                    cur = self.class_defs.get(&cls).and_then(|i| i.parent.clone());
                }
                self.body.push(Instruction::LocalGet(obj));
                return Ok(());
            }
        }
        // Llamada a funciÃƒÂ³n nativa (extensiÃƒÂ³n): import `env.<sym>__<sig>@<lib>`.
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(idx) = self.native_indexes.get(name) {
                for a in &c.args {
                    self.emit_expression(a)?;
                }
                self.body.push(Instruction::Call(*idx));
                return Ok(());
            }
        }
        // MÃƒÂ©todos de primitivos (callee MemberAccess)
        if let Expression::MemberAccess(member) = &*c.callee {
            // `super.m(args)` Ã¢â€ â€™ call directo al mÃƒÂ©todo del padre (sin vtable).
            if let Expression::Identifier(sn, _) = &*member.object {
                if sn == "super" {
                    if let Some(cur) = &self.current_class {
                        if let Some(parent) =
                            self.class_defs.get(cur).and_then(|i| i.parent.clone())
                        {
                            // `super.main(...)` Ã¢â€ â€™ ctor del padre (ClassDef.ctor se
                            // emite como `__ctor`). `super.metodo(...)` Ã¢â€ â€™ mÃƒÂ©todo.
                            let key = if member.member == "main" {
                                format!("{}::__ctor", parent)
                            } else {
                                format!("{}::{}", parent, member.member)
                            };
                            if let Some(idx) = self.func_indexes.get(&key) {
                                self.body.push(Instruction::LocalGet(0)); // me
                                for a in &c.args {
                                    self.emit_expression(a)?;
                                }
                                self.emit_call_site(&c.span);
                                self.body.push(Instruction::Call(*idx));
                                return Ok(());
                            }
                        }
                    }
                    return Err(crate::error::ClsError::CompileError(
                        "super solo se puede usar dentro de mÃƒÂ©todos de clase (JIT)".to_string(),
                    ));
                }
            }
            // MÃƒÂ³dulos stdlib: math / json / fs
            if let Expression::Identifier(obj_name, _) = &*member.object {
                if obj_name == "math" {
                    return self.emit_math_call(member, c);
                }
                if obj_name == "json" {
                    if member.member == "parse" {
                        self.emit_expression(&c.args[0])?;
                        self.host.call(HostFn::JsonParse, &mut self.body);
                        return Ok(());
                    }
                    if member.member == "stringify" {
                        let t = self
                            .types
                            .get(&expr_span(&c.args[0]))
                            .cloned()
                            .unwrap_or(Type::Any);
                        // Objeto de clase: __toJson si lo define; si no Ã¢â€ â€™ "null" (paridad walker).
                        if let Type::Named(cn, _) = &t {
                            if self.class_defs.contains_key(cn.as_str()) {
                                if self.emit_class_method("__toJson", &c.args[0])? {
                                    return Ok(());
                                }
                                self.emit_expression(&c.args[0])?;
                                self.body.push(Instruction::Drop);
                                let n = self.intern_string("null");
                                self.emit_load_str(n);
                                return Ok(());
                            }
                            // struct/enum sin serializaciÃƒÂ³n Ã¢â€ â€™ "null" (paridad walker).
                            if self.struct_defs.contains_key(cn.as_str())
                                || self.enum_defs.contains_key(cn.as_str())
                            {
                                self.emit_expression(&c.args[0])?;
                                self.body.push(Instruction::Drop);
                                let n = self.intern_string("null");
                                self.emit_load_str(n);
                                return Ok(());
                            }
                        }
                        // Shape Ã¢â€ â€™ stringify inline (json.stringify({x:1}) Ã¢â€ â€™ '{"x":1}').
                        if let Type::Shape(fields) = &t {
                            return self.emit_shape_to_json_string(&c.args[0], fields);
                        }
                        self.emit_expression(&c.args[0])?;
                        let kind = match t {
                            Type::Record(_, _) => 1,
                            Type::Array(_) => 2,
                            _ => 0,
                        };
                        self.body.push(Instruction::I64Const(kind));
                        self.host.call(HostFn::JsonStringify, &mut self.body);
                        return Ok(());
                    }
                }
                if obj_name == "fs" {
                    return self.emit_fs_call(member, c);
                }
                if obj_name == "http" {
                    return self.emit_http_call(member, c);
                }
                if obj_name == "os" {
                    return self.emit_os_call(member, c);
                }
                if obj_name == "path" {
                    return self.emit_path_call(member, c);
                }
                if obj_name == "process" {
                    return self.emit_process_call(member, c);
                }
                if obj_name == "time" {
                    return self.emit_time_call(member, c);
                }
                if obj_name == "random" {
                    return self.emit_random_call(member, c);
                }
                // `Clase.metodo()` con mÃƒÂ©todo static Ã¢â€ â€™ call directo (sin me).
                if self.class_defs.contains_key(obj_name.as_str()) {
                    let skey = format!("{}::__s__{}", obj_name, member.member);
                    if let Some(&idx) = self.func_indexes.get(&skey) {
                        for a in &c.args {
                            self.emit_expression(a)?;
                        }
                        self.emit_call_site(&c.span);
                        self.body.push(Instruction::Call(idx));
                        return Ok(());
                    }
                }
            }
            let obj_ty = self
                .types
                .get(&expr_span(&member.object))
                .cloned()
                .unwrap_or(Type::Any);
            match obj_ty {
                Type::Tuple(_) => match member.member.as_str() {
                    "join" => return self.emit_tuple_join(member, c),
                    _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                },
                Type::String => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "upper" | "lower" | "trim" => {
                            let h = match member.member.as_str() {
                                "upper" => HostFn::StrUpper,
                                "lower" => HostFn::StrLower,
                                _ => HostFn::StrTrim,
                            };
                            self.host.call(h, &mut self.body);
                            return Ok(());
                        }
                        "contains" | "startsWith" | "endsWith" => {
                            self.emit_expression(&c.args[0])?;
                            let h = match member.member.as_str() {
                                "contains" => HostFn::StrContains,
                                "startsWith" => HostFn::StrStartsWith,
                                _ => HostFn::StrEndsWith,
                            };
                            self.host.call(h, &mut self.body);
                            return Ok(());
                        }
                        "isEmpty" => {
                            self.host.call(HostFn::StrIsEmpty, &mut self.body);
                            return Ok(());
                        }
                        "toString" => return Ok(()),
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Array(_) => {
                    let elem_ty = self.array_elem_was_type(&member.object)?;
                    let elem_size = elem_size_bytes(elem_ty);
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "push" => {
                            self.emit_expression(&c.args[0])?;
                            self.elem_to_bits(&c.args[0], elem_ty)?;
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrPush, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "pop" => {
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrPop, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "shift" => {
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrShift, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "unshift" => {
                            self.emit_expression(&c.args[0])?;
                            self.elem_to_bits(&c.args[0], elem_ty)?;
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrUnshift, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "reverse" => {
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrReverse, &mut self.body);
                            self.writeback_array(&member.object)?;
                            return Ok(());
                        }
                        "indexOf" => {
                            self.emit_expression(&c.args[0])?;
                            self.elem_to_bits(&c.args[0], elem_ty)?;
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrIndexOf, &mut self.body);
                            return Ok(());
                        }
                        "includes" => {
                            self.emit_expression(&c.args[0])?;
                            self.elem_to_bits(&c.args[0], elem_ty)?;
                            self.body.push(Instruction::I64Const(elem_size));
                            self.host.call(HostFn::ArrIncludes, &mut self.body);
                            return Ok(());
                        }
                        "join" => {
                            self.emit_expression(&c.args[0])?;
                            self.body.push(Instruction::I64Const(elem_size));
                            let cls_t = self.array_elem_cls_type(&member.object)?;
                            self.body.push(Instruction::I64Const(arr_kind_code(&cls_t)));
                            self.host.call(HostFn::ArrJoin, &mut self.body);
                            return Ok(());
                        }
                        "map" => return self.emit_array_map(member, c, elem_ty, elem_size),
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Record(_, _) => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "has" => {
                            self.emit_expression(&c.args[0])?;
                            self.host.call(HostFn::RecordHas, &mut self.body);
                            return Ok(());
                        }
                        "keys" => {
                            self.host.call(HostFn::RecordKeys, &mut self.body);
                            return Ok(());
                        }
                        "values" => {
                            self.host.call(HostFn::RecordValues, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Shape(fields) => {
                    match member.member.as_str() {
                        "has" => {
                            // Compile-time: si la clave (literal) estÃƒÂ¡ en el shape.
                            let has = match &c.args[0] {
                                Expression::Literal(l)
                                    if matches!(l.kind, LiteralKind::String(_)) =>
                                {
                                    match &l.kind {
                                        LiteralKind::String(k) => {
                                            fields.iter().any(|(n, _)| *n == *k)
                                        }
                                        _ => false,
                                    }
                                }
                                _ => true, // clave dinÃƒÂ¡mica Ã¢â€ â€™ se asume que puede existir
                            };
                            self.body
                                .push(Instruction::I32Const(if has { 1 } else { 0 }));
                            return Ok(());
                        }
                        "keys" => {
                            // Construir array<String> con las keys del shape.
                            let mut sorted: Vec<&String> = fields.iter().map(|(n, _)| n).collect();
                            sorted.sort();
                            let n = sorted.len() as i64;
                            let es = 8i64;
                            self.body.push(Instruction::I64Const(n));
                            self.body.push(Instruction::I64Const(es));
                            self.body.push(Instruction::I64Mul);
                            self.body.push(Instruction::I64Const(16));
                            self.body.push(Instruction::I64Add);
                            let alloc = self.func_indexes["__alloc"];
                            self.body.push(Instruction::Call(alloc));
                            let ptr = self.fresh_local();
                            self.body.push(Instruction::LocalSet(ptr));
                            self.body.push(Instruction::LocalGet(ptr));
                            self.body.push(Instruction::I64Const(n));
                            self.emit_i64_store(0);
                            self.body.push(Instruction::LocalGet(ptr));
                            self.body.push(Instruction::I64Const(n));
                            self.emit_i64_store(8);
                            for (i, k) in sorted.iter().enumerate() {
                                self.body.push(Instruction::LocalGet(ptr));
                                self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
                                self.body.push(Instruction::I64Add);
                                self.body.push(Instruction::I32WrapI64);
                                let s = self.intern_string(k);
                                self.emit_load_str(s);
                                self.body.push(Instruction::I64Store(MemArg {
                                    offset: 0,
                                    align: 3,
                                    memory_index: 0,
                                }));
                            }
                            self.body.push(Instruction::LocalGet(ptr));
                            return Ok(());
                        }
                        "values" => {
                            // Construir array con los valores (segÃƒÂºn el tipo de cada campo).
                            self.emit_expression(&member.object)?;
                            let ptr = self.fresh_local();
                            self.body.push(Instruction::LocalSet(ptr));
                            let layout = self.shape_layout(&fields)?;
                            let mut ordered: Vec<&(String, WasTy, i64)> = layout.iter().collect();
                            ordered.sort_by(|a, b| a.0.cmp(&b.0));
                            let n = fields.len() as i64;
                            let es = 8i64;
                            self.body.push(Instruction::I64Const(n));
                            self.body.push(Instruction::I64Const(es));
                            self.body.push(Instruction::I64Mul);
                            self.body.push(Instruction::I64Const(16));
                            self.body.push(Instruction::I64Add);
                            let alloc = self.func_indexes["__alloc"];
                            self.body.push(Instruction::Call(alloc));
                            let arr = self.fresh_local();
                            self.body.push(Instruction::LocalSet(arr));
                            self.body.push(Instruction::LocalGet(arr));
                            self.body.push(Instruction::I64Const(n));
                            self.emit_i64_store(0);
                            self.body.push(Instruction::LocalGet(arr));
                            self.body.push(Instruction::I64Const(n));
                            self.emit_i64_store(8);
                            for (i, (_, w, off)) in ordered.iter().enumerate() {
                                self.body.push(Instruction::LocalGet(arr));
                                self.body.push(Instruction::I64Const(16 + (i as i64) * 8));
                                self.body.push(Instruction::I64Add);
                                self.body.push(Instruction::I32WrapI64);
                                self.body.push(Instruction::LocalGet(ptr));
                                self.body.push(Instruction::I64Const(*off));
                                self.body.push(Instruction::I64Add);
                                self.body.push(Instruction::I32WrapI64);
                                match *w {
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
                                // bits a i64 (f64 Ã¢â€ â€™ reinterpret; i32 Ã¢â€ â€™ extend)
                                match *w {
                                    WasTy::F64 => self.body.push(Instruction::I64ReinterpretF64),
                                    WasTy::I32 => self.body.push(Instruction::I64ExtendI32U),
                                    WasTy::I64 => {}
                                }
                                self.body.push(Instruction::I64Store(MemArg {
                                    offset: 0,
                                    align: 3,
                                    memory_index: 0,
                                }));
                            }
                            self.body.push(Instruction::LocalGet(arr));
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Named(name, _) => {
                    if let Some(info) = self.class_defs.get(name.as_str()) {
                        let method_slot = info
                            .methods
                            .iter()
                            .position(|m| *m == member.member)
                            .ok_or_else(|| {
                                crate::error::ClsError::compile_at(
                                    &format!(
                                        "El mÃƒÂ©todo '{}' no existe en la clase '{}'",
                                        member.member, name
                                    ),
                                    &member.span,
                                )
                            })? as u32;
                        // Visibilidad del mÃƒÂ©todo: private/protected desde fuera Ã¢â€ â€™ error.
                        // Se resuelve subiendo por ancestors (un mÃƒÂ©todo puede venir
                        // del padre sin override).
                        let mut vis_cls = name.to_string();
                        let vis = loop {
                            if let Some(v) = self
                                .class_defs
                                .get(&vis_cls)
                                .and_then(|i| i.method_vis.get(&member.member))
                            {
                                break Some(*v);
                            }
                            match self
                                .class_defs
                                .get(&vis_cls)
                                .and_then(|i| i.parent.clone())
                            {
                                Some(p) => vis_cls = p,
                                None => break None,
                            }
                        };
                        if let Some(v) = vis {
                            self.check_method_access(&name, &member.member, v, &member.span)?;
                        }
                        // MÃƒÂ©todo heredado sin override: buscar el ÃƒÂ­ndice en la clase
                        // que lo declara (no fallar con "MÃƒÂ©todo sin tipo WASM").
                        let mut fn_cls = name.to_string();
                        let ty = loop {
                            let key = format!("{}::{}", fn_cls, member.member);
                            if let Some(t) = self.method_type_indexes.get(&key) {
                                break *t;
                            }
                            match self
                                .class_defs
                                .get(&fn_cls)
                                .and_then(|i| i.parent.clone())
                            {
                                Some(p) => fn_cls = p,
                                None => {
                                    return Err(crate::error::ClsError::compile_at(
                                        &format!(
                                            "El mÃƒÂ©todo '{}' no existe en la clase '{}'",
                                            member.member, name
                                        ),
                                        &member.span,
                                    ))
                                }
                            }
                        };
                        let obj_tmp = self.fresh_local();
                        self.emit_expression(&member.object)?;
                        self.body.push(Instruction::LocalSet(obj_tmp));
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        for a in &c.args {
                            self.emit_expression(a)?;
                        }
                        // slot = vtable(obj[0]) + method_slot
                        self.body.push(Instruction::LocalGet(obj_tmp));
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::I64Load(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                        self.body.push(Instruction::I64Const(method_slot as i64));
                        self.body.push(Instruction::I64Add);
                        self.body.push(Instruction::I32WrapI64);
                        self.body.push(Instruction::CallIndirect {
                            type_index: ty,
                            table_index: 0,
                        });
                        return Ok(());
                    }
                    return Err(self.unsupported_expr(&Expression::Call(c.clone())));
                }
                Type::Int => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "toString" => {
                            self.host.call(HostFn::StrInt, &mut self.body);
                            return Ok(());
                        }
                        "abs" => {
                            self.host.call(HostFn::IntAbs, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Float => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "toString" => {
                            self.host.call(HostFn::StrFloat, &mut self.body);
                            return Ok(());
                        }
                        "abs" => {
                            self.host.call(HostFn::FloatAbs, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Bool => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "toString" => {
                            self.host.call(HostFn::StrBool, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                Type::Char => {
                    self.emit_expression(&member.object)?;
                    match member.member.as_str() {
                        "toString" => {
                            self.host.call(HostFn::StrChar, &mut self.body);
                            return Ok(());
                        }
                        _ => return Err(self.unsupported_expr(&Expression::Call(c.clone()))),
                    }
                }
                _ => {}
            }
        }
        if let Expression::Identifier(name, _) = &*c.callee {
            match name.as_str() {
                "throw" => {
                    // throw(msg) Ã¢â€ â€™ excepciÃƒÂ³n CLS (tag con payload msg + span).
                    if !self.exceptions {
                        return Err(crate::error::ClsError::compile_at(
                            "'throw' no soportado en este runtime: el backend se compilÃƒÂ³ sin \
                             excepciones WASM (wasmi).",
                            &c.span,
                        ));
                    }
                    if let Some(arg0) = c.args.first() {
                        self.emit_expression(arg0)?;
                        self.emit_to_string(arg0)?;
                    } else {
                        let s = self.intern_string("error");
                        self.emit_load_str(s);
                    }
                    let packed = ((c.span.start_line as i64) << 32) | (c.span.start_col as i64);
                    self.body.push(Instruction::I64Const(packed));
                    self.body.push(Instruction::Throw(self.tag_idx));
                    return Ok(());
                }
                "print" => {
                    for arg in &c.args {
                        self.emit_print_arg(arg)?;
                    }
                    self.host.call(HostFn::PrintEnd, &mut self.body);
                    return Ok(());
                }
                "len" => {
                    let arg = &c.args[0];
                    // Magic __len: clase con __len Ã¢â€ â€™ call sin args (paridad walker).
                    if self.emit_class_method("__len", arg)? {
                        return Ok(());
                    }
                    self.emit_expression(arg)?;
                    // String Ã¢â€ â€™ decodifica el pack (ptr<<32|len); array/tuple/record
                    // Ã¢â€ â€™ lee el header. Despachar por el tipo del argumento.
                    let t = self.types.get(&expr_span(arg)).cloned().unwrap_or(Type::Any);
                    match t {
                        Type::String => {
                            self.host.call(HostFn::StrLength, &mut self.body);
                        }
                        Type::Record(_, _) | Type::Shape(_) => {
                            self.host.call(HostFn::RecordLen, &mut self.body);
                        }
                        _ => self.emit_array_len(),
                    }
                    return Ok(());
                }
                "toString" => {
                    let arg = &c.args[0];
                    self.emit_expression(arg)?;
                    self.emit_to_string(arg)?;
                    return Ok(());
                }
                "str" => {
                    let arg = &c.args[0];
                    self.emit_expression(arg)?;
                    self.emit_to_string(arg)?;
                    return Ok(());
                }
                "input" => {
                    self.host.call(HostFn::Input, &mut self.body);
                    return Ok(());
                }
                "int" => {
                    let arg = &c.args[0];
                    // Magic __int: clase con __int Ã¢â€ â€™ call sin args (paridad walker).
                    if self.emit_class_method("__int", arg)? {
                        return Ok(());
                    }
                    self.emit_expression(arg)?;
                    self.emit_to_int(arg)?;
                    return Ok(());
                }
                "float" => {
                    let arg = &c.args[0];
                    // Magic __float: clase con __float Ã¢â€ â€™ call sin args.
                    if self.emit_class_method("__float", arg)? {
                        return Ok(());
                    }
                    self.emit_expression(arg)?;
                    self.emit_to_float(arg)?;
                    return Ok(());
                }
                "bool" => {
                    let arg = &c.args[0];
                    // Magic __bool: clase con __bool Ã¢â€ â€™ call sin args.
                    if self.emit_class_method("__bool", arg)? {
                        return Ok(());
                    }
                    self.emit_expression(arg)?;
                    self.emit_to_bool(arg)?;
                    return Ok(());
                }
                "type" => {
                    let arg = &c.args[0];
                    // Si la clase define __type Ã¢â€ â€™ llamarla (paridad con el walker).
                    if self.emit_class_method("__type", arg)? {
                        return Ok(());
                    }
                    let span = expr_span(arg);
                    let t = self.types.get(&span).cloned().unwrap_or(Type::Any);
                    // type_name del walker: claseÃ¢â€ â€™"Object", structÃ¢â€ â€™"Struct", enumÃ¢â€ â€™"Enum".
                    let name = match &t {
                        Type::Named(cn, _) if self.class_defs.contains_key(cn.as_str()) => "Object",
                        Type::Named(cn, _) if self.struct_defs.contains_key(cn.as_str()) => {
                            "Struct"
                        }
                        Type::Named(cn, _) if self.enum_defs.contains_key(cn.as_str()) => "Enum",
                        Type::Named(_, _) => "Object",
                        _ => type_name_str(&t),
                    };
                    let idx = self.intern_string(name);
                    self.emit_load_str(idx);
                    return Ok(());
                }
                "now" => {
                    self.host.call(HostFn::Now, &mut self.body);
                    return Ok(());
                }
                "exit" => {
                    self.emit_expression(&c.args[0])?;
                    self.host.call(HostFn::Exit, &mut self.body);
                    return Ok(());
                }
                "sleep" => {
                    self.emit_expression(&c.args[0])?;
                    self.host.call(HostFn::Sleep, &mut self.body);
                    return Ok(());
                }
                _ => {}
            }
        }
        // `x::f(...)` Ã¢â‚¬â€ mÃƒÂ³dulo/namespace importado: call directo a `x::f`.
        if let Expression::NamespaceAccess(ns, member, _) = &*c.callee {
            let key = format!("{}::{}", ns, member);
            if let Some(fidx) = self.func_indexes.get(&key).copied() {
                self.body.push(Instruction::I64Const(0)); // __capturas
                for arg in &c.args {
                    self.emit_expression(arg)?;
                }
                self.emit_call_site(&c.span);
                self.body.push(Instruction::Call(fidx));
                return Ok(());
            }
            return Err(crate::error::ClsError::compile_at(
                &format!(
                    "El miembro '{}' no existe o no se exporta en el mÃƒÂ³dulo '{}' (fase de emisiÃƒÂ³n).",
                    member, ns
                ),
                &expr_span(&c.callee),
            ));
        }
        if let Expression::Identifier(name, _) = &*c.callee {
            if let Some(fidx) = self.func_indexes.get(name).copied() {
                // Firma uniforme (B5): las funciones CLS top-level reciben
                // __capturas (0) como primer arg. Internas y main no.
                if !name.starts_with("__") && name != "main" {
                    self.body.push(Instruction::I64Const(0));
                }
                for arg in &c.args {
                    self.emit_expression(arg)?;
                }
                // Args faltantes Ã¢â€ â€™ valores por defecto (en el call site)
                if let Some(defaults) = self.func_defaults.get(name) {
                    let provided = c.args.len();
                    for d in defaults.iter().skip(provided) {
                        match d {
                            Some(expr) => self.emit_expression(expr)?,
                            None => self.body.push(Instruction::I64Const(0)),
                        }
                    }
                }
                self.emit_call_site(&c.span);
                self.body.push(Instruction::Call(fidx));
                return Ok(());
            }
            // FunciÃƒÂ³n host del nodo (intrinsic): canal `env.host_call(id, ptr, n)`.
            if let Some(intr) = self.intrinsics.get(name) {
                self.emit_host_call(intr, c)?;
                return Ok(());
            }
        }
        // FunciÃƒÂ³n como valor (variable con handle) Ã¢â€ â€™ call_indirect por tipo.
        let callee_ty = self.types.get(&expr_span(&c.callee)).cloned();
        if let Some(Type::Fun(params, ret)) = callee_ty {
            let mut pv: Vec<ValType> = Vec::new();
            for t in &params {
                pv.push(was_type(t)?.val_type());
            }
            let rv: Vec<ValType> = match &*ret {
                Type::Void => vec![],
                r => vec![was_type(r)?.val_type()],
            };
            // Firma uniforme (B5): closure = [capturas(i64), params...].
            // Toda funciÃƒÂ³n CLS (top-level y arrows) se compila con el capturas
            // como primer param. El dispatch usa tag-bit: impar = closure (lee
            // el ptr de capturas del handle en memoria); par = funciÃƒÂ³n simple
            // (capturas = 0 literal, sin handle).
            let mut pv_closure = vec![ValType::I64];
            pv_closure.extend(pv.iter().copied());
            let tidx_closure = self.register_func_type(pv_closure, rv.clone());
            // v = eval(callee); valor con tag (par = simple, impar = closure).
            self.emit_expression(&c.callee)?;
            let v = self.fresh_local();
            self.body.push(Instruction::LocalSet(v));
            // block $done (resultado del call) Ã¢â€ â€™ cada rama hace call_indirect + br.
            let ret_block = if rv.is_empty() {
                BlockType::Empty
            } else {
                BlockType::Result(rv[0])
            };
            // tag = v & 1 Ã¢â€ â€™ condiciÃƒÂ³n del if (impar = closure). Convertir a i32.
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64And);
            self.body.push(Instruction::I32WrapI64);
            self.block_depth += 1;
            self.body.push(Instruction::If(ret_block));
            // Rama closure (impar): ptr = v>>1; capturas = handle[8] (aplanado).
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I64Const(8));
            self.body.push(Instruction::I64Add);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            let caps_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(caps_tmp));
            // push [capturas, args..., tabla_idx]
            self.body.push(Instruction::LocalGet(caps_tmp));
            for arg in &c.args {
                self.emit_expression(arg)?;
            }
            // Params faltantes Ã¢â€ â€™ Null (0), como el walker (default o Null).
            for _ in c.args.len()..params.len() {
                self.body.push(Instruction::I64Const(0));
            }
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I32WrapI64);
            self.body.push(Instruction::I64Load(MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
            self.body.push(Instruction::I32WrapI64);
            self.emit_call_site(&c.span);
            self.body.push(Instruction::CallIndirect {
                type_index: tidx_closure,
                table_index: 0,
            });
            self.body.push(Instruction::Else);
            // Rama simple (par): tabla_idx = v>>1; push [capturas=0, args..., tabla_idx].
            self.body.push(Instruction::I64Const(0));
            for arg in &c.args {
                self.emit_expression(arg)?;
            }
            for _ in c.args.len()..params.len() {
                self.body.push(Instruction::I64Const(0));
            }
            self.body.push(Instruction::LocalGet(v));
            self.body.push(Instruction::I64Const(1));
            self.body.push(Instruction::I64ShrU);
            self.body.push(Instruction::I32WrapI64);
            self.emit_call_site(&c.span);
            self.body.push(Instruction::CallIndirect {
                type_index: tidx_closure,
                table_index: 0,
            });
            self.body.push(Instruction::End);
            self.block_depth -= 1;
            return Ok(());
        }
        // Magic __call: el callee es un objeto de clase con __call Ã¢â€ â€™
        // obj(args...) = __call(obj, args...) (paridad walker interpreter.rs:1644).
        let callee_ty = self.types.get(&expr_span(&c.callee)).cloned();
        if let Some(cn) = self.class_magic_method(&callee_ty, "__call") {
            let _ = self.magic_ret_was(&cn, "__call")?;
            self.emit_expression(&c.callee)?;
            let obj_tmp = self.fresh_local();
            self.body.push(Instruction::LocalSet(obj_tmp));
            self.emit_class_method_call_on("__call", &cn, obj_tmp, &c.args)?;
            return Ok(());
        }
        // Objeto sin __call invocado como funciÃƒÂ³n Ã¢â€ â€™ error claro (paridad walker).
        if let Some(Type::Named(cn2, _)) = callee_ty {
            if self.class_defs.contains_key(cn2.as_str()) {
                return Err(crate::error::ClsError::compile_at(
                    &format!(
                        "El objeto de tipo '{}' no es callable (falta __call)",
                        cn2
                    ),
                    &c.span,
                ));
            }
        }
        Err(self.unsupported_expr(&Expression::Call(c.clone())))
    }

}