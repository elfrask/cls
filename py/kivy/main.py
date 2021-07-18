import kivy
kivy.require('1.10.0') # replace with your current kivy version !

from kivy.app import App
from kivy.uix.text import Text


class MyApp(App):

    def build(self):
    	#print(dir(gui))
    	return Text(text='Hello world')


if __name__ == '__main__':
    MyApp().run()