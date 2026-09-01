//! Funciones host (`env.*`) que el nodo JIT debe implementar: el emisor
//! referencia cada host por su import name y su firma WASM.

use wasm_encoder::ValType;
/// Funciones host (`env.*`) que el nodo JIT debe implementar.
///
/// Solo las que el emisor llama directo (I/O, reloj, closures, CMX, Any, JSON,
/// módulos del nodo). Las operaciones de strings/arrays/records/math/parse se
/// resuelven a internals fusionadas `__intr_*` (0 cruces de frontera); no hay
/// variantes host para ellas (poda Fase 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostFn {
    PrintInt,
    PrintFloat,
    PrintBool,
    PrintChar,
    PrintStr,
    PrintEnd,
    Now,
    Exit,
    Sleep,
    Trap,
    Input,
    AnyToBool,
    MathRandom,
    JsonStringify,
    JsonParse,
    FsExists,
    FsCwd,
    FsReadFile,
    FsWriteFile,
    FsListDir,
    FsMkdir,
    FsRm,
    HttpGet,
    HttpPost,
    CmxNew,
    CmxSetProp,
    CmxAddChild,
    CmxToString,
    PrintAny,
    AnyMember,
    AnyIndex,
    FnHandle,
    FnToString,
    FnEnter,
    FnExit,
    CallSite,
    /// Canal genérico de funciones host del nodo: `host_call(id, ptr, n) -> i64`
    /// con args empaquetados `[n:i64][(val:i64, tag:i64)*n]`.
    HostCall,
    // Módulo os
    OsPlatform,
    OsArch,
    OsVersion,
    OsHostname,
    OsHome,
    OsTempdir,
    OsCpus,
    OsPid,
    OsUptime,
    OsEnv,
    OsSep,
    OsIsWindows,
    OsIsUnix,
    // Módulo path
    PathJoin,
    PathBasename,
    PathDirname,
    PathExtname,
    PathResolve,
    PathNormalize,
    PathIsAbsolute,
    PathSep,
    // Módulo process
    ProcessArgs,
    ProcessCwd,
    ProcessEnv,
    ProcessExit,
    ProcessPid,
    ProcessPlatform,
    ProcessTitle,
    // Módulo time
    TimeNow,
    TimeSeconds,
    TimeIso,
    TimeDate,
    TimeClock,
    TimeYear,
    TimeMonth,
    TimeDay,
    TimeHour,
    TimeMinute,
    TimeSecond,
    TimeSleep,
    // Módulo random
    RandomRandom,
    RandomInt,
    RandomFloat,
    RandomUuid,
    // Módulo net (sockets TCP)
    // NetListen/Accept/Recv/Send/Close/LastError eliminados (dev-2):
    // el módulo `net` ya no existe en el runtime. Los sockets deben venir
    // de `extension`+`when` en el .clsx del usuario.
}

impl HostFn {
    pub fn import_name(&self) -> &'static str {
        use HostFn::*;
        match self {
            PrintInt => "print_int",
            PrintFloat => "print_float",
            PrintBool => "print_bool",
            PrintChar => "print_char",
            PrintStr => "print_str",
            PrintEnd => "print_end",
            Now => "now",
            Exit => "exit",
            Sleep => "sleep",
            Trap => "trap",
            Input => "input",
            AnyToBool => "any_to_bool",
            MathRandom => "math_random",
            JsonStringify => "json_stringify",
            JsonParse => "json_parse",
            FsExists => "fs_exists",
            FsCwd => "fs_cwd",
            FsReadFile => "fs_read_file",
            FsWriteFile => "fs_write_file",
            FsListDir => "fs_list_dir",
            FsMkdir => "fs_mkdir",
            FsRm => "fs_rm",
            HttpGet => "http_get",
            HttpPost => "http_post",
            CmxNew => "cmx_new",
            CmxSetProp => "cmx_set_prop",
            CmxAddChild => "cmx_add_child",
            CmxToString => "cmx_to_string",
            PrintAny => "print_any",
            AnyMember => "any_member",
            AnyIndex => "any_index",
            FnHandle => "fn_handle",
            FnToString => "fn_to_string",
            FnEnter => "fn_enter",
            FnExit => "fn_exit",
            CallSite => "fn_call_site",
            HostCall => "host_call",
            OsPlatform => "os_platform",
            OsArch => "os_arch",
            OsVersion => "os_version",
            OsHostname => "os_hostname",
            OsHome => "os_home",
            OsTempdir => "os_tempdir",
            OsCpus => "os_cpus",
            OsPid => "os_pid",
            OsUptime => "os_uptime",
            OsEnv => "os_env",
            OsSep => "os_sep",
            OsIsWindows => "os_is_windows",
            OsIsUnix => "os_is_unix",
            PathJoin => "path_join",
            PathBasename => "path_basename",
            PathDirname => "path_dirname",
            PathExtname => "path_extname",
            PathResolve => "path_resolve",
            PathNormalize => "path_normalize",
            PathIsAbsolute => "path_is_absolute",
            PathSep => "path_sep",
            ProcessArgs => "process_args",
            ProcessCwd => "process_cwd",
            ProcessEnv => "process_env",
            ProcessExit => "process_exit",
            ProcessPid => "process_pid",
            ProcessPlatform => "process_platform",
            ProcessTitle => "process_title",
            TimeNow => "time_now",
            TimeSeconds => "time_seconds",
            TimeIso => "time_iso",
            TimeDate => "time_date",
            TimeClock => "time_clock",
            TimeYear => "time_year",
            TimeMonth => "time_month",
            TimeDay => "time_day",
            TimeHour => "time_hour",
            TimeMinute => "time_minute",
            TimeSecond => "time_second",
            TimeSleep => "time_sleep",
            RandomRandom => "random_random",
            RandomInt => "random_int",
            RandomFloat => "random_float",
            RandomUuid => "random_uuid",
        }
    }

    pub(super) fn signature(&self) -> (Vec<ValType>, Vec<ValType>) {
        use HostFn::*;
        let i64p = vec![ValType::I64];
        match self {
            PrintInt | PrintStr => (i64p.clone(), vec![]),
            PrintFloat => (vec![ValType::F64], vec![]),
            PrintBool | PrintChar => (vec![ValType::I32], vec![]),
            PrintEnd => (vec![], vec![]),
            Now => (vec![], vec![ValType::I64]),
            Exit | Sleep => (i64p.clone(), vec![]),
            Trap => (vec![ValType::I64, ValType::I64], vec![]),
            Input => (vec![], vec![ValType::I64]),
            AnyToBool => (vec![ValType::I64, ValType::I64], vec![ValType::I32]),
            MathRandom => (vec![], vec![ValType::F64]),
            JsonStringify => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            JsonParse => (i64p.clone(), vec![ValType::I64]),
            FsExists => (i64p.clone(), vec![ValType::I32]),
            FsCwd => (vec![], vec![ValType::I64]),
            FsReadFile => (i64p.clone(), vec![ValType::I64]),
            FsWriteFile => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            FsListDir | FsMkdir | FsRm => (i64p.clone(), vec![ValType::I64]),
            HttpGet => (i64p.clone(), vec![ValType::I64]),
            HttpPost => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            CmxNew => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            CmxSetProp => (
                vec![ValType::I64, ValType::I64, ValType::I64, ValType::I64],
                vec![ValType::I64],
            ),
            CmxAddChild => (
                vec![ValType::I64, ValType::I64, ValType::I64],
                vec![ValType::I64],
            ),
            CmxToString => (i64p.clone(), vec![ValType::I64]),
            PrintAny => (vec![ValType::I64, ValType::I64], vec![]),
            AnyMember => (
                vec![ValType::I64, ValType::I64, ValType::I64],
                vec![ValType::I64, ValType::I64],
            ),
            AnyIndex => (
                vec![ValType::I64, ValType::I64, ValType::I64],
                vec![ValType::I64, ValType::I64],
            ),
            FnHandle => (
                vec![ValType::I64, ValType::I64, ValType::I64],
                vec![ValType::I64],
            ),
            FnToString => (i64p.clone(), vec![ValType::I64]),
            FnEnter => (
                vec![ValType::I64, ValType::I64, ValType::I64],
                vec![],
            ),
            FnExit => (vec![], vec![]),
            CallSite => (
                vec![ValType::I64, ValType::I64],
                vec![],
            ),
            HostCall => (vec![ValType::I64, ValType::I64, ValType::I64], vec![ValType::I64]),
            // Módulo os: sin args -> i64 (string) o i32 (bool); env(key) -> i64
            OsPlatform | OsArch | OsVersion | OsHostname | OsHome | OsTempdir
            | OsCpus | OsPid | OsUptime | OsSep => (vec![], vec![ValType::I64]),
            OsEnv => (i64p.clone(), vec![ValType::I64]),
            OsIsWindows | OsIsUnix => (vec![], vec![ValType::I32]),
            // Módulo path
            PathBasename | PathDirname | PathExtname | PathResolve | PathNormalize => {
                (i64p.clone(), vec![ValType::I64])
            }
            PathJoin => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            PathIsAbsolute => (i64p.clone(), vec![ValType::I32]),
            PathSep => (vec![], vec![ValType::I64]),
            // Módulo process
            ProcessArgs | ProcessCwd | ProcessPid | ProcessPlatform | ProcessTitle => {
                (vec![], vec![ValType::I64])
            }
            ProcessEnv => (i64p.clone(), vec![ValType::I64]),
            ProcessExit => (i64p.clone(), vec![]),
            // Módulo time
            TimeNow | TimeSeconds | TimeIso | TimeDate | TimeClock | TimeYear
            | TimeMonth | TimeDay | TimeHour | TimeMinute | TimeSecond => {
                (vec![], vec![ValType::I64])
            }
            TimeSleep => (i64p.clone(), vec![]),
            // Módulo random
            RandomRandom => (vec![], vec![ValType::F64]),
            RandomInt => (vec![ValType::I64, ValType::I64], vec![ValType::I64]),
            RandomFloat => (vec![ValType::F64, ValType::F64], vec![ValType::F64]),
            RandomUuid => (vec![], vec![ValType::I64]),
            // Módulo net: listen(port)->handle, accept(handle)->sock,
            // recv(sock,max)->String, send(sock,data)->n, close(sock)->0, lastError()->String
            // net_* eliminados (dev-2): ver comentario arriba.
        }
    }
}
