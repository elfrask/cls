from llvmlite import ir

# 1. Definir el módulo
module = ir.Module(name="mi_programa")

# 2. Definir una función principal (como un 'main' en C)
func_type = ir.FunctionType(ir.IntType(32), [])  # Retorna i32, sin argumentos
func = ir.Function(module, func_type, name="main")

# 3. Crear un bloque de código
block = func.append_basic_block(name="entry")
builder = ir.IRBuilder(block)

# 4. Construir las instrucciones
val_1 = ir.Constant(ir.IntType(32), 5)
val_2 = ir.Constant(ir.IntType(32), 10)

# Generar la instrucción de suma (ADD)
resultado = builder.add(val_1, val_2, name="tmp_suma")

# Generar la instrucción de retorno
builder.ret(resultado)

# El módulo 'module' ahora contiene la IR de LLVM
llvm_ir_string = str(module)
print(llvm_ir_string)