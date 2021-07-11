app.index = 0
try:
    for v_i in (app.values['int'](1)  ,  app.values['int'](2)  ,  app.values['int'](3)  ,  app.values['int'](4)):
        app.index = 27
        try:
            v_print  (v_i)

        except Exception as e:
            app.error(e, 'ErrorExecute', 27)


        pass

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

