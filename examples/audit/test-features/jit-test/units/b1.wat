clx.exe : [JIT] M+¦dulo WASM inv+ílido para 'C:\Users\Frask\AppData\Local\Temp\opencode\b1.clsx':
En línea: 1 Carácter: 196
+ ...  $_.Line }; & "target\debug\clx.exe" run --jit "C:\Users\Frask\AppDat ...
+                 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: ([JIT] M+¦dulo W...ncode\b1.clsx'::String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError
 
WebAssembly translation error

Caused by:
    Invalid input WebAssembly code at offset 1390: unknown local 2: local index out of bounds
--- WAT ---
(module
  (type (;0;) (func (param i64)))
  (type (;1;) (func (param f64)))
  (type (;2;) (func (param i32)))
  (type (;3;) (func (param i32)))
  (type (;4;) (func (param i64)))
  (type (;5;) (func))
  (type (;6;) (func (result i64)))
  (type (;7;) (func (param i64)))
  (type (;8;) (func (param i64)))
  (type (;9;) (func (param i64)))
  (type (;10;) (func (param i64) (result i64)))
  (type (;11;) (func (param i64) (result f64)))
  (type (;12;) (func (param i64) (result i32)))
  (type (;13;) (func (param i64 i64) (result i64)))
  (type (;14;) (func (param i64) (result i64)))
  (type (;15;) (func (param f64) (result i64)))
  (type (;16;) (func (param i32) (result i64)))
  (type (;17;) (func (param i32) (result i64)))
  (type (;18;) (func (param i64 i64) (result i64)))
  (type (;19;) (func (param f64 f64) (result f64)))
  (type (;20;) (func (result i64)))
  (type (;21;) (func (param i64) (result i64)))
  (type (;22;) (func (param i64) (result i64)))
  (type (;23;) (func (param i64) (result i64)))
  (type (;24;) (func (param i64 i64) (result i32)))
  (type (;25;) (func (param i64 i64) (result i32)))
  (type (;26;) (func (param i64 i64) (result i32)))
  (type (;27;) (func (param i64) (result i32)))
  (type (;28;) (func (param i64) (result i64)))
  (type (;29;) (func (param i64) (result i64)))
  (type (;30;) (func (param f64) (result f64)))
  (type (;31;) (func (param i64 i64 i64) (result i64)))
  (type (;32;) (func (param i64 i64) (result i64)))
  (type (;33;) (func (param i64 i64) (result i64)))
  (type (;34;) (func (param i64 i64 i64) (result i64)))
  (type (;35;) (func (param i64 i64 i64) (result i64)))
  (type (;36;) (func (param i64 i64 i64) (result i32)))
  (type (;37;) (func (param i64 i64 i64 i64) (result i64)))
  (type (;38;) (func (param i64 i64) (result i64)))
  (type (;39;) (func (param f64) (result f64)))
  (type (;40;) (func (param f64 f64) (result f64)))
  (type (;41;) (func (param f64 f64) (result f64)))
  (type (;42;) (func (param f64 f64) (result f64)))
  (type (;43;) (func (param f64) (result f64)))
  (type (;44;) (func (param f64) (result f64)))
  (type (;45;) (func (param f64) (result f64)))
  (type (;46;) (func (result f64)))
  (type (;47;) (func (param f64) (result f64)))
  (type (;48;) (func (param f64) (result f64)))
  (type (;49;) (func (param f64) (result f64)))
  (type (;50;) (func (param f64) (result f64)))
  (type (;51;) (func (param i64 i64) (result i64)))
  (type (;52;) (func (param i64) (result i64)))
  (type (;53;) (func (param i64) (result i32)))
  (type (;54;) (func (result i64)))
  (type (;55;) (func (param i64) (result i64)))
  (type (;56;) (func (param i64 i64) (result i64)))
  (type (;57;) (func (param i64) (result i64)))
  (type (;58;) (func (param i64) (result i64)))
  (type (;59;) (func (param i64) (result i64)))
  (type (;60;) (func))
  (type (;61;) (func (param i64) (result i64)))
  (type (;62;) (func (param i64) (result i64)))
  (type (;63;) (func (result i64)))
  (type (;64;) (func (param i64) (result i64)))
  (import "env" "print_int" (func (;0;) (type 0)))
  (import "env" "print_float" (func (;1;) (type 1)))
  (import "env" "print_bool" (func (;2;) (type 2)))
  (import "env" "print_char" (func (;3;) (type 3)))
  (import "env" "print_str" (func (;4;) (type 4)))
  (import "env" "print_end" (func (;5;) (type 5)))
  (import "env" "now" (func (;6;) (type 6)))
  (import "env" "exit" (func (;7;) (type 7)))
  (import "env" "sleep" (func (;8;) (type 8)))
  (import "env" "trap" (func (;9;) (type 9)))
  (import "env" "parse_int" (func (;10;) (type 10)))
  (import "env" "parse_float" (func (;11;) (type 11)))
  (import "env" "parse_bool" (func (;12;) (type 12)))
  (import "env" "str_concat" (func (;13;) (type 13)))
  (import "env" "str_int" (func (;14;) (type 14)))
  (import "env" "str_float" (func (;15;) (type 15)))
  (import "env" "str_bool" (func (;16;) (type 16)))
  (import "env" "str_char" (func (;17;) (type 17)))
  (import "env" "pow_num" (func (;18;) (type 18)))
  (import "env" "fmod" (func (;19;) (type 19)))
  (import "env" "input" (func (;20;) (type 20)))
  (import "env" "str_upper" (func (;21;) (type 21)))
  (import "env" "str_lower" (func (;22;) (type 22)))
  (import "env" "str_trim" (func (;23;) (type 23)))
  (import "env" "str_contains" (func (;24;) (type 24)))
  (import "env" "str_starts_with" (func (;25;) (type 25)))
  (import "env" "str_ends_with" (func (;26;) (type 26)))
  (import "env" "str_is_empty" (func (;27;) (type 27)))
  (import "env" "str_length" (func (;28;) (type 28)))
  (import "env" "int_abs" (func (;29;) (type 29)))
  (import "env" "float_abs" (func (;30;) (type 30)))
  (import "env" "arr_push" (func (;31;) (type 31)))
  (import "env" "arr_pop" (func (;32;) (type 32)))
  (import "env" "arr_shift" (func (;33;) (type 33)))
  (import "env" "arr_unshift" (func (;34;) (type 34)))
  (import "env" "arr_index_of" (func (;35;) (type 35)))
  (import "env" "arr_includes" (func (;36;) (type 36)))
  (import "env" "arr_join" (func (;37;) (type 37)))
  (import "env" "arr_reverse" (func (;38;) (type 38)))
  (import "env" "math_sqrt" (func (;39;) (type 39)))
  (import "env" "math_pow" (func (;40;) (type 40)))
  (import "env" "math_min" (func (;41;) (type 41)))
  (import "env" "math_max" (func (;42;) (type 42)))
  (import "env" "math_floor" (func (;43;) (type 43)))
  (import "env" "math_ceil" (func (;44;) (type 44)))
  (import "env" "math_round" (func (;45;) (type 45)))
  (import "env" "math_random" (func (;46;) (type 46)))
  (import "env" "math_sin" (func (;47;) (type 47)))
  (import "env" "math_cos" (func (;48;) (type 48)))
  (import "env" "math_tan" (func (;49;) (type 49)))
  (import "env" "math_log" (func (;50;) (type 50)))
  (import "env" "math_range" (func (;51;) (type 51)))
  (import "env" "json_stringify" (func (;52;) (type 52)))
  (import "env" "fs_exists" (func (;53;) (type 53)))
  (import "env" "fs_cwd" (func (;54;) (type 54)))
  (import "env" "fs_read_file" (func (;55;) (type 55)))
  (import "env" "fs_write_file" (func (;56;) (type 56)))
  (import "env" "fs_list_dir" (func (;57;) (type 57)))
  (import "env" "fs_mkdir" (func (;58;) (type 58)))
  (import "env" "fs_rm" (func (;59;) (type 59)))
  (func (;60;) (type 60)
    (local i64 i64)
    global.get 0
    local.set 1
    local.get 1
    local.get 0
    i64.add
    i64.const 8
    i64.add
    i64.const -8
    i64.and
    local.set 2
    block ;; label = @1
      local.get 2
      memory.size
      i64.extend_i32_u
      i64.const 65536
      i64.mul
      i64.le_u
      br_if 0 (;@1;)
      i32.const 16
      memory.grow
      drop
    end
    local.get 2
    global.set 0
    local.get 1
  )
  (func (;61;) (type 61) (param i64) (result i64)
    (local i64 i64 i64)
    local.get 0
    i64.const 8
    i64.mul
    local.set 1
    local.get 1
    i32.wrap_i64
    i32.load
    i64.extend_i32_u
    local.set 2
    local.get 1
    i64.const 4
    i64.add
    i32.wrap_i64
    i32.load
    i64.extend_i32_u
    local.set 3
    local.get 2
    i64.const 32
    i64.shl
    local.get 3
    i64.or
  )
  (func (;62;) (type 62) (param i64) (result i64)
    i64.const 0
    global.set 1
    i64.const 2
    global.set 2
  )
  (func (;63;) (type 63) (result i64)
    global.get 1
    i64.const 1
    i64.add
    global.set 1
    global.get 1
    drop
    global.get 1
    return
  )
  (func (;64;) (type 64) (param i64) (result i64)
    (local i64)
    call 60
    i64.const 0
    call 62
    call 4
    global.get 1
    call 0
    call 5
    call 63
    drop
    call 63
    drop
    i64.const 1
    call 62
    call 4
    global.get 1
    call 0
    call 5
    i64.const 2
    call 62
    call 4
    global.get 2
    call 0
    call 5
    global.get 1
    i64.const 100
    i64.add
    global.set 1
    global.get 1
    drop
    i64.const 3
    call 62
    call 4
    global.get 1
    call 0
    call 5
    i64.const 5
    local.set 1
    i64.const 4
    call 62
    call 4
    local.get 1
    call 0
    call 5
    i64.const 0
    return
  )
  (memory (;0;) 1)
  (global (;0;) (mut i64) i64.const 1048576)
  (global (;1;) (mut i64) i64.const 0)
  (global (;2;) (mut i64) i64.const 0)
  (export "main" (func 64))
  (export "alloc" (func 61))
  (export "memory" (memory 0))
  (data (;0;) (i32.const 0) "(\00\00\00\11\00\00\009\00\00\00\09\00\00\00B\00\00\00\07\00\00\00I\00\00\00\0f\00\00\00X\
00\00\00\06\00\00\00contador inicial:contador:factor:contador final:local:")
)

