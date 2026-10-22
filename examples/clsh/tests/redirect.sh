echo hello > clsh_t_f.txt
echo world >> clsh_t_f.txt
read a < clsh_t_f.txt
echo "a=$a"
echo again > clsh_t_f.txt
read b < clsh_t_f.txt
echo "b=$b"
nosuchcmd 2> clsh_t_e.txt
echo done
echo "$?"
