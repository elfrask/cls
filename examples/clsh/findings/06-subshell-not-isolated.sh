# REPRO (gap del lenguaje / shell): `( ... )` NO aisla el estado.
# POSIX: el grupo corre en un subshell; los cambios de variable no salen.
# clsh : run_subshell re-usa el MISMO Sh -> las variables se filtran.
#
#   dash: inner  /  outer
#   clsh: inner  /  inner
x=outer
( x=inner; echo "$x" )
echo "$x"
