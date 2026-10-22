if true; then
  echo yes
else
  echo no
fi
if false; then echo a; elif true; then echo b; else echo c; fi
keep=1
while [ -n "$keep" ]; do
  echo once
  keep=
done
for x in a b c; do
  echo "x=$x"
done
