import clsengine as cls
import sys
import os


def main():
    app:cls.appcls = cls.appcls(0)

    print(app.desline(sys.argv[1]))

    pass



if __name__ == "__main__":
    main()