[ 1 -eq 1 ] && echo eq
[ 1 -ne 2 ] && echo ne
[ 2 -lt 3 ] && echo lt
[ 3 -le 3 ] && echo le
[ 4 -gt 2 ] && echo gt
[ 4 -ge 4 ] && echo ge
[ -z "" ] && echo z
[ -n "x" ] && echo n
[ "a" = "a" ] && echo seq
[ "a" != "b" ] && echo sne
[ ! -z "x" ] && echo notz
[ ! -n "" ] && echo notn
test 5 -eq 5 && echo and-test
