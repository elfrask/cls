app.index = 1
try:
    v_lista = ([app.values['int'](1)  ,  app.values['int'](2)  ,  app.values['int'](3)  ,  app.values['int'](4)  ,  app.values['int'](5)  ,  app.values['int'](6)  ,  app.values['int'](7)  ,  app.values['int'](8)  ,  app.values['int'](9)])

except Exception as e:
    app.error(e, 'ErrorExecute', 1)

app.index = 31
try:
    v_i = (app.values['int'](0))
    while (True):
        if not (v_i  <=  v_len  (v_lista)): break
        app.index = 69
        try:
            v_print  (v_i)

        except Exception as e:
            app.error(e, 'ErrorExecute', 69)

        app.index = 83
        try:
            v_i  +=1

        except Exception as e:
            app.error(e, 'ErrorExecute', 83)


        v_i  +=1
        pass

except Exception as e:
    app.error(e, 'ErrorExecute', 31)

