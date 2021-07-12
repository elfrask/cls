class v_main():
    def __init__(self, *arg):
        f_rt = (v_Any)
        v_me = self
        try:
            v_n = arg[0]
        except:
            v_n = (None)
        app.dim(v_n, v_Any)
        app.index = 46
        try:
            v_me.nombre = (v_n)

        except Exception as e:
            app.error(e, 'ErrorExecute', 46)


        pass
    
    def saludo(self, *arg):
        f_rt = (v_Any)
        v_me = self
        app.index = 99
        try:
            v_print  (app.values['str']("Saludos,")  ,  v_me.nombre)

        except Exception as e:
            app.error(e, 'ErrorExecute', 99)


        pass
    

    pass
app.index = 138
try:
    v_main  (app.values['str']("Carlos")) .saludo  ()

except Exception as e:
    app.error(e, 'ErrorExecute', 138)

