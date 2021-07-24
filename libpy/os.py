import os
import platform


def osname():
    return platform.system()


export.osname = osname 