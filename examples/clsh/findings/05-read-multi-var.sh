# REPRO (bug del shell): `read a b` con varios nombres.
# POSIX: el ULTIMO nombre recibe el resto de la linea (sin re-split).
# clsh : reparte un campo (ya spliteado por IFS) por nombre -> b pierde "three".
#
# dash: printf? no hace falta; este archivo usa echo + read.
#   dash  docs: x=one   y=two three
#   clsh      : x=one   y=two
echo "one two three" > clsh_f_read.txt
read x y < clsh_f_read.txt
echo "x=$x"
echo "y=$y"
