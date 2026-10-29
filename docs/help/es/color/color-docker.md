# El panel Color

El panel [[docker.color]] (<kbd>Shift+F11</kbd>) mezcla un color y lo aplica
al relleno o al contorno de la selección. La herramienta
[[tool.outline_color]] (<kbd>Shift+F12</kbd>) también lo abre. Ábralo desde
[[menu.window > menu.window.dockers]].

![El panel Color](../../img/color-color-docker.png)

## Elegir un color {#choose}

1. Seleccione objetos. El panel parte del color de relleno del primero.
2. Mezcle el nuevo color en una de las vistas:
   - [[docker.color_viewers]]: haga clic o arrastre en el cuadrado para
     ajustar la saturación (de izquierda a derecha) y el brillo (de abajo
     arriba), y en la franja de al lado para ajustar el tono;
   - [[docker.color_sliders]]: un deslizador por cada componente del
     [[docker.model]]: RGB, CMYK (opción predeterminada), HSB, HSL, Lab,
     Escala de grises o YIQ;
   - [[docker.color_palettes]]: las muestras de una paleta.

   Los tres botones de arriba muestran u ocultan cada vista; puede mostrarse
   cualquier combinación. También puede escribir un valor `#rrggbb`.
3. Haga clic en [[docker.fill]] o [[docker.outline]], en la parte inferior del
   panel, para aplicar el nuevo color.
   El candado ([[docker.lock_colors]]) une relleno y contorno, de modo que
   cualquiera de los dos botones aplica el color a ambos. El botón "..."
   ([[docker.more_actions]]) despliega las demás acciones: [[docker.no_fill]],
   [[docker.no_outline_action]], [[docker.swap_colors]],
   [[docker.add_to_document_palette]], [[docker.add_to_spot_colors]] y
   [[docker.add_to_color_styles]].

El color anterior y el nuevo se muestran uno al lado del otro. Cuando el nuevo
color no se puede imprimir con exactitud con tintas CMYK, el panel muestra un
aviso.

## Tomar una muestra de color {#sample}

Haga clic en [[docker.eyedropper]] y luego en cualquier punto del dibujo para
tomar el color de ese lugar. Después haga clic en [[docker.use_sampled]] para
convertirlo en el nuevo color. Vea también
[Cuentagotas](eyedroppers.md).

## Tintas planas {#spot-inks}

Debajo de los botones, [[docker.spot_inks]] enumera las tintas planas del
dibujo; vea [Colores planos](spot-colors.md).
