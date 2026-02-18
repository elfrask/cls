import os
import platform
import sys


def osname(): return platform.system()

def username(): return os.getlogin()

def run(*a): return os.system(*a)

def kill(*a): return os.kill(*a)


export.osname = osname
export.username = username
export.run = run
export.kill = kill
export.getpid = lambda x: os.getpid() 
export.cpu_count = os.cpu_count 
export.argv = sys.argv
