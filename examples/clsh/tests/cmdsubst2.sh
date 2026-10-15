echo $(echo a; echo b)
echo "$(echo x)"
echo "nested: $(echo $(echo y))"
