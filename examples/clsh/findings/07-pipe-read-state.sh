# REPRO (gap del lenguaje / shell): el lado derecho de un pipe corre en el
# MISMO Sh que el izquierdo, y su stdin es el stdout capturado (buffer).
# POSIX: cada stage del pipe corre en un proceso aparte; `read` en el lado
# derecho NO afecta al shell padre.
#
#   dash: x=      (vacio: `read` corre en el subshell del pipe)
#   clsh: x=hello (el pipe alimenta me.input y `read` corre en el padre)
echo hello | read x
echo "x=$x"
