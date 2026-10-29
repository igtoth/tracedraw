# Copiar colores y atributos con los cuentagotas

Las herramientas de cuentagotas toman un color, o todo un conjunto de
atributos, de un objeto y se lo dan a otros. Ambas funcionan en dos pasos, que
se muestran en la barra de propiedades: **seleccionar** (tomar una muestra) y
**aplicar** (pintar con ella). Tras tomar una muestra, la herramienta pasa a
aplicar; haga clic en el botón de seleccionar para tomar otra muestra.
<kbd>Esc</kbd> descarta la muestra y vuelve a la herramienta [[tool.pick]].

El puntero es una pipeta mientras la herramienta toma muestras y un bote de
pintura mientras aplica. Las imágenes dan su color incluso dentro de un grupo o
un marco.

## Cuentagotas de color {#color-eyedropper}

1. Elija la herramienta [[tool.color_eyedropper]].
2. Haga clic en un objeto para tomar su color: el color de un relleno
   uniforme, el primer color de un degradado o el color del contorno. En un
   mapa de bits, toma el color de los píxeles bajo el puntero, promediado
   según el tamaño de muestra fijado en la barra de propiedades (1 por 1, 2
   por 2 o 5 por 5 píxeles). <kbd>Shift</kbd>+clic toma primero el color del
   contorno.
3. Haga clic en objetos para rellenarlos con ese color; <kbd>Shift</kbd>+clic
   para colorear su contorno (un objeto sin contorno recibe uno fino). Hacer
   clic dentro de una forma cerrada vacía la rellena.

La barra de propiedades muestra el color tomado.

Mientras mueve el puntero para tomar un color, un pequeño cuadro a su lado
muestra el color que hay debajo: una muestra, el nombre del color si es un
color de la paleta o una tinta plana, sus valores en el modo de color del
dibujo (CMYK o RGB) y su valor hexadecimal. El cuentagotas de un selector de
color muestra el mismo cuadro.

## Cuentagotas de atributos {#attributes-eyedropper}

La herramienta [[tool.attributes_eyedropper]] copia varias propiedades a la
vez de un objeto a otros.

1. Elija la herramienta [[tool.attributes_eyedropper]].
2. En la barra de propiedades, elija qué copiar:
   - **Propiedades**: contorno, relleno y texto (la fuente, el tamaño y el
     estilo; el texto en sí se conserva). Todas activadas de forma
     predeterminada.
   - **Transformaciones**: tamaño, rotación y posición. Desactivadas de forma
     predeterminada.
   - **Efectos**: transparencia, sombra y efectos dinámicos. Desactivados de
     forma predeterminada.
3. Haga clic en el objeto del que copiar.
4. Haga clic en cada objeto al que copiar. <kbd>Shift</kbd>+clic toma en su
   lugar la muestra de otro objeto.

Cada clic es un paso de deshacer.
