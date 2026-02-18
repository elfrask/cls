import threading
import subprocess
import time

def main():
    time.sleep(1)
    print("hola 2")
    pass

a = threading.Timer(0.0, main, [], {}).start()
print("hola 1")