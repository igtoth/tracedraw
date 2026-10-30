# Calcar mapas de bits

El calco convierte un mapa de bits en formas vectoriales que puede editar,
recolorear y escalar sin perder calidad: un logotipo escaneado, un boceto,
una foto.

## Calco rápido {#quick-trace}

Seleccione un mapa de bits y elija
[[menu.bitmaps > menu.bitmaps.quick_trace]]. Usa el ajuste preestablecido
elegido en la página [[options.bitmap_tracing]] de las opciones globales
(Clipart al principio).

[[menu.bitmaps.repeat_last_trace]] calca de nuevo el mapa de bits seleccionado como el último calco, sin cuadro de diálogo, y [[menu.bitmaps.last_trace_settings]] abre el cuadro de diálogo con sus ajustes. El menú contextual de un mapa de bits también tiene estos comandos, junto con [[menu.bitmaps.centerline_trace]] y [[menu.bitmaps.outline_trace]].

## El cuadro de diálogo Calcar mapa de bits {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] y
[[menu.bitmaps.centerline_trace]] muestran ajustes preestablecidos; elija
uno para abrir el cuadro de diálogo.

- **Calco por contorno** rellena las áreas de color con formas:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] y [[menu.bitmaps.trace_high_quality]].
- **Calco por línea central** sigue las líneas con curvas abiertas:
  [[menu.bitmaps.trace_technical]] y [[menu.bitmaps.trace_line_drawing]].

![El cuadro de diálogo Calcar mapa de bits](../../img/bitmaps-tracing.png)

El cuadro de diálogo es una ventana que puede cambiar de tamaño y maximizar.
La vista previa ocupa su lado izquierdo, los ajustes están en un panel a la
derecha, y el calco se rehace cada vez que cambia un ajuste: no hay nada que
pulsar para ver el resultado. Bajo la vista previa, una línea cuenta las
**curvas**, los **nodos** y los **colores** del resultado. Haga clic en
[[dialog.ok]] para colocar las formas sobre el mapa de bits, las más oscuras
primero, o en [[dialog.cancel]] para dejar el mapa de bits como está.

## La vista previa {#preview}

La lista [[dialog.trace_preview_label]] elige lo que muestra la vista
previa:

- [[dialog.trace_view_before_after]]: el mapa de bits arriba y su calco
  abajo, o [[dialog.view_side_by_side]]; [[dialog.before]] muestra solo el
  mapa de bits.
- [[dialog.trace_view_large]]: solo el calco, tan grande como la vista
  previa.
- [[dialog.trace_view_wireframe]]: las curvas del resultado dibujadas como
  líneas finas sobre el mapa de bits. Junto a la lista aparece el control
  [[dialog.trace_overlay]]: fija cuánto se ve el mapa de bits bajo las
  líneas, de 0 (solo las líneas) a 100 (el mapa de bits tal cual), 80 al
  principio.

Los botones que siguen a la lista ajustan la imagen a la ventana, alejan,
acercan y desplazan. La rueda amplía alrededor del puntero y un arrastre
mueve la imagen; ambos paneles se mueven juntos. Las partes transparentes
muestran un tablero de ajedrez.

## Ajustes {#settings}

La pestaña [[dialog.trace_tab_settings]] contiene:

- [[dialog.trace_type]] Contorno o Línea central, y
  [[dialog.trace_image_type]] el ajuste preestablecido. Elegir uno pone los
  controles de abajo en los valores que convienen a ese tipo de imagen.
- [[dialog.detail]] (0 a 100): más detalle conserva áreas más pequeñas y
  líneas más finas; menos detalle suaviza motas y grano.
- [[dialog.smoothing]] (0 a 100, 25 al principio): curvas más suaves con
  menos nodos.
- [[dialog.corner_smoothness]] (0 a 100): esquinas más redondeadas.
- [[dialog.trace_remove]]: lo que se deja fuera del resultado. **Color de
  fondo** quita el fondo donde llega al borde de la imagen, **Color
  completo de la imagen** quita cada área de ese color, **Ninguno** calca
  todo. [[dialog.trace_remove_auto]] encuentra el color de fondo a lo largo
  del borde de la imagen; [[dialog.trace_remove_specify]] usa el color de
  la muestra, que puede cambiar o tomar de la vista previa con el
  cuentagotas de al lado.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (activado al
  principio) hace una sola curva de cada área de un color; desactivado,
  cada mancha separada es una curva propia. [[dialog.trace_remove_overlap]]
  mantiene cada forma en sus propios píxeles; desactivado, las formas se
  extienden un poco bajo las de encima, para que no se vean huecos entre
  vecinas. [[dialog.trace_group_by_color]] pone las curvas de cada color en
  un grupo propio; está disponible cuando las áreas de un color no están ya
  combinadas.
- [[dialog.delete_original]]: el mapa de bits desaparece al colocar el
  resultado.

Los ajustes que no se aplican al tipo de calco actual aparecen atenuados:
un calco por línea central solo lee el detalle y el suavizado.

## Colores {#colors}

La pestaña [[dialog.trace_tab_colors]] fija el [[dialog.color_mode]] (RGB,
escala de grises, o blanco y negro) y el [[dialog.trace_number_of_colors]]
(2 a 256), y lista los colores del resultado con el número de curvas de
cada uno. Marque dos o más colores y haga clic en
[[dialog.trace_merge_colors]] para unirlos en el primero; haga clic en la
muestra de un color para dibujar sus curvas en otro color. Cambiar un
ajuste que calca de nuevo la imagen olvida estas ediciones, que pertenecían
al resultado anterior.

## Restablecer, deshacer y rehacer {#steps}

Bajo el panel de ajustes, [[dialog.reset]] devuelve los ajustes del ajuste
preestablecido, y las dos flechas retroceden y avanzan por cada cambio
hecho en el cuadro de diálogo: cada control soltado, elección en una lista,
casilla, edición de color y restablecimiento es un paso. Un cambio hecho
después de deshacer descarta los pasos deshechos.

[[dialog.trace_options]] abre la página [[options.bitmap_tracing]] de las
opciones globales, donde elige el ajuste preestablecido del calco rápido y
las opciones con las que empieza cada calco, y luego vuelve al cuadro de
diálogo. El botón [[dialog.help_button]] abre esta página.
