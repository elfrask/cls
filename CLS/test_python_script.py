def main():
    ma = "Hola mundo"
    def meta():
        print(ma)
        print(globals().get("ma", locals()["ma"]))
        pass
    meta()
    pass
main()
