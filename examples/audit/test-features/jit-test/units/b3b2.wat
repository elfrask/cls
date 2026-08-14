clx.exe : --- WAT ---
En línea: 1 Carácter: 310
+ ... MP_WAT="1"; & "target\debug\clx.exe" run --jit "C:\Users\Frask\AppDat ...
+                 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: (--- WAT ---:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError
 
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
  (type (;60;) (func (param i64) (result i64)))
  (type (;61;) (func (param i64 i64 i64) (result i64)))
  (type (;62;) (func (param i64 i64) (result i64)))
  (type (;63;) (func (param i64 i64) (result i32)))
  (type (;64;) (func (param i64) (result i64)))
  (type (;65;) (func (param i64) (result i64)))
  (type (;66;) (func (param i64) (result i64)))
  (type (;67;) (func (param i64) (result i64)))
  (type (;68;) (func (param i64 i64) (result i64)))
  (type (;69;) (func (param i64) (result i64)))
  (type (;70;) (func (param i64) (result i64)))
  (type (;71;) (func (param i64)))
  (type (;72;) (func (param i64) (result i64)))
  (type (;73;) (func (param i64) (result i64)))
  (type (;74;) (func (param i64) (result i64)))
  (type (;75;) (func (param i64) (result i64)))
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
a hablar: generico
p hablar: generico
p nombre: 0
p presentar: guau
a is Animal: true
p is Animal: true
p is Perro: true
a is Perro: false
  (import "env" "json_stringify" (func (;52;) (type 52)))
  (import "env" "fs_exists" (func (;53;) (type 53)))
  (import "env" "fs_cwd" (func (;54;) (type 54)))
  (import "env" "fs_read_file" (func (;55;) (type 55)))
  (import "env" "fs_write_file" (func (;56;) (type 56)))
  (import "env" "fs_list_dir" (func (;57;) (type 57)))
  (import "env" "fs_mkdir" (func (;58;) (type 58)))
  (import "env" "fs_rm" (func (;59;) (type 59)))
  (import "env" "record_new" (func (;60;) (type 60)))
  (import "env" "record_set" (func (;61;) (type 61)))
  (import "env" "record_get" (func (;62;) (type 62)))
  (import "env" "record_has" (func (;63;) (type 63)))
  (import "env" "record_len" (func (;64;) (type 64)))
  (import "env" "record_keys" (func (;65;) (type 65)))
  (import "env" "record_values" (func (;66;) (type 66)))
  (import "env" "http_get" (func (;67;) (type 67)))
  (import "env" "http_post" (func (;68;) (type 68)))
  (func (;69;) (type 69) (param i64) (result i64)
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
  (func (;70;) (type 70) (param i64) (result i64)
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
  (func (;71;) (type 71) (param i64)
    (local i64 i64)
    local.get 0
    local.set 1
    i64.const 0
    call 70
    local.set 2
    local.get 1
    i64.const 16
    i64.add
    i32.wrap_i64
    local.get 2
    i64.store
    local.get 2
    drop
  )
  (func (;72;) (type 72) (param i64) (result i64)
    i64.const 1
    call 70
    return
  )
  (func (;73;) (type 73) (param i64) (result i64)
    i64.const 2
    call 70
    return
  )
  (func (;74;) (type 74) (param i64) (result i64)
    (local i64)
    local.get 0
    call 72
    i64.const 3
    call 70
    call 13
    local.get 0
    local.set 1
    local.get 1
    local.get 1
    i32.wrap_i64
    i64.load
    i64.const 0
    i64.add
    i32.wrap_i64
    call_indirect (type 73)
    call 13
    return
  )
  (func (;75;) (type 75) (param i64) (result i64)
    (local i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
    i64.const 24
    call 69
    local.set 2
    local.get 2
    i64.const 0
    local.set 3
    i32.wrap_i64
    local.get 3
    i64.store
    local.get 2
    i64.const 0
    local.set 4
    i32.wrap_i64
    local.get 4
    i64.store offset=8
    local.get 2
    i64.const 16
    i64.add
    i32.wrap_i64
    i64.const 0
    i64.store
    local.get 2
    call 71
    local.get 2
    local.set 1
    i64.const 24
    call 69
    local.set 6
    local.get 6
    i64.const 0
    local.set 7
    i32.wrap_i64
    local.get 7
    i64.store
    local.get 6
    i64.const 1
    local.set 8
    i32.wrap_i64
    local.get 8
    i64.store offset=8
    local.get 6
    i64.const 16
    i64.add
    i32.wrap_i64
    i64.const 0
    i64.store
    local.get 6
    local.get 6
    local.set 5
    i64.const 4
    call 70
    call 4
    local.get 1
    local.set 9
    local.get 9
    local.get 9
    i32.wrap_i64
    i64.load
    i64.const 0
    i64.add
    i32.wrap_i64
    call_indirect (type 72)
    call 4
    call 5
    i64.const 5
    call 70
    call 4
    local.get 5
    local.set 10
    local.get 10
    local.get 10
    i32.wrap_i64
    i64.load
    i64.const 0
    i64.add
    i32.wrap_i64
    call_indirect (type 73)
    call 4
    call 5
    i64.const 6
    call 70
    call 4
    local.get 5
    i64.const 16
    i64.add
    i32.wrap_i64
    i64.load
    call 0
    call 5
    i64.const 7
    call 70
    call 4
    local.get 5
    local.set 11
    local.get 11
    local.get 11
    i32.wrap_i64
    i64.load
    i64.const 1
    i64.add
    i32.wrap_i64
    call_indirect (type 74)
    call 4
    call 5
    i64.const 8
    call 70
    call 4
    local.get 1
    local.set 12
    local.get 12
    i32.wrap_i64
    i64.load offset=8
    local.set 13
    local.get 13
    i64.const 0
    i64.eq
    local.get 13
    i64.const 1
    i64.eq
    i32.or
    call 2
    call 5
    i64.const 9
    call 70
    call 4
    local.get 5
    local.set 14
    local.get 14
    i32.wrap_i64
    i64.load offset=8
    local.set 15
    local.get 15
    i64.const 0
    i64.eq
    local.get 15
    i64.const 1
    i64.eq
    i32.or
    call 2
    call 5
    i64.const 10
    call 70
    call 4
    local.get 5
    local.set 16
    local.get 16
    i32.wrap_i64
    i64.load offset=8
    local.set 17
    local.get 17
    i64.const 1
    i64.eq
    call 2
    call 5
    i64.const 11
    call 70
    call 4
    local.get 1
    local.set 18
    local.get 18
    i32.wrap_i64
    i64.load offset=8
    local.set 19
    local.get 19
    i64.const 1
    i64.eq
    call 2
    call 5
    i64.const 0
    return
  )
  (table (;0;) 3 funcref)
  (memory (;0;) 1)
  (global (;0;) (mut i64) i64.const 1048576)
  (export "main" (func 75))
  (export "alloc" (func 69))
  (export "memory" (memory 0))
  (elem (;0;) (i32.const 0) func 72 73 74)
  (data (;0;) (i32.const 0) "`\00\00\00\06\00\00\00f\00\00\00\08\00\00\00n\00\00\00\04\00\00\00r\00\00\00\03\00\00\00u\
00\00\00\09\00\00\00~\00\00\00\09\00\00\00\87\00\00\00\09\00\00\00\90\00\00\00\0c\00\00\00\9c\00\00\00\0c\00\00\00\a8\0
0\00\00\0c\00\00\00\b4\00\00\00\0b\00\00\00\bf\00\00\00\0b\00\00\00animalgenericoguau y a hablar:p hablar:p nombre:p 
presentar:a is Animal:p is Animal:p is Perro:a is Perro:")
)

