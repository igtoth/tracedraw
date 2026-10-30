# Rellenos

Un relleno pinta el interior de un objeto cerrado. TraceDraw tiene seis tipos
de relleno: [[docker.fill_none]], [[docker.fill_uniform]] (un color plano),
[[docker.fill_fountain]] (colores que se funden unos con otros),
[[docker.fill_pattern]], [[docker.fill_texture]] y [[docker.fill_mesh]]
(colores que fluyen entre los nodos de una cuadrícula).

## Formas de rellenar un objeto {#ways}

- Haga clic en un color de la paleta para dar a la selección un relleno
  uniforme.
- Use la sección [[docker.fill]] del panel [[docker.properties]]
  (<kbd>F11</kbd>): elija el tipo de relleno con la fila de imágenes y luego
  sus ajustes.
- Use la herramienta [[tool.interactive_fill]] (<kbd>G</kbd>) para crear
  degradados sobre el propio objeto.
- Use la herramienta [[tool.area_fill]] para rellenar un área encerrada por
  varios objetos.
- Use la herramienta [[tool.mesh_fill]] (<kbd>M</kbd>) para un relleno de
  malla.

Sin nada seleccionado, el relleno que elija pasa a ser el relleno de los
objetos nuevos.

## Rellenos uniformes {#uniform}

Un relleno uniforme es un solo color, en cualquier modelo de color. Elíjalo en
la paleta, en el panel [[docker.color]] (vea
[El panel Color](color-docker.md)) o en la sección de relleno del panel
[[docker.properties]].

## Rellenos degradados {#gradient}

Un degradado funde dos o más colores. Sus ajustes, en la sección de relleno
del panel [[docker.properties]]:

- El tipo: [[fill.linear]] (a lo largo de una línea), [[fill.radial]] (del
  centro hacia fuera), [[fill.conical]] (alrededor del centro),
  [[fill.square]] (en cuadrados crecientes) o [[fill.shaped]] (siguiendo el
  contorno del objeto, del borde hacia dentro).
- [[docker.gradient_presets]]: degradados ya preparados de los que partir.
- Las **paradas**: los colores y su posición a lo largo del degradado, de 2 a
  64. Seleccione una parada para cambiar su color o su posición;
  [[docker.add_stop]] y [[docker.remove_stop]] añaden y quitan paradas.
  [[docker.import_gradient]] toma los colores de un archivo de degradado
  `.ggr`.
- El **ángulo** de los degradados lineales, cónicos y cuadrados, el
  [[docker.edge_pad]] (cuánto de cada extremo es color sólido) y el
  desplazamiento del centro de los degradados radiales, cónicos y cuadrados.
- [[docker.repeat]]: [[docker.repeat_none]], [[docker.repeat_sawtooth]] (los
  colores vuelven a empezar desde el principio) o [[docker.repeat_mirror]]
  (van y vuelven), con el número de [[docker.repeat_count]].
- [[docker.bilinear]]: los colores van del primero al último y vuelven.
- [[docker.invert_colours]]: los colores van en orden inverso.

Una franja debajo del tipo muestra toda la secuencia de colores tal como se
dibujará.

## El cuadro de diálogo Editar relleno {#edit-fill}

<kbd>F11</kbd> abre el cuadro de diálogo [[edit_fill.title]] con el relleno de
la selección. Para un degradado muestra una vista previa, el
[[edit_fill.fill_picker]], el tipo, la repetición y la banda de colores: haga
clic en un nodo para elegirlo, arrástrelo para moverlo y haga doble clic para
añadir o quitar uno. Bajo la banda: el color, la transparencia, la posición y
el [[edit_fill.midpoint]] del nodo, e [[edit_fill.reverse]].
[[edit_fill.transform]] tiene la rotación, el centro y el relleno del borde.
**Aceptar** aplica lo que cambió, con [[edit_fill.overprint]]. Mientras el cuadro de diálogo está abierto, la selección muestra el relleno a medida que lo cambia; **Cancelar** o <kbd>Esc</kbd> restaura el relleno anterior.

![El cuadro de diálogo Editar relleno](../../img/color-fills-edit-fill.png)

## La herramienta Relleno interactivo {#interactive-fill}

La herramienta [[tool.interactive_fill]] (<kbd>G</kbd>) edita los rellenos
sobre el objeto.

1. Elija la herramienta y arrastre sobre un objeto: obtendrá un degradado
   lineal desde donde pulsa hasta donde suelta. Mantenga pulsado
   <kbd>Ctrl</kbd> para limitar el ángulo a pasos de 15 grados. Un clic da el
   relleno predeterminado.
2. Arrastre los nodos sobre el objeto para cambiar el degradado: los dos nodos
   cuadrados de un degradado lineal giran su dirección y cambian su longitud;
   el nodo de un degradado radial, cónico o cuadrado mueve su centro.
3. Arrastre un color de la paleta sobre un nodo para cambiar su color, o sobre
   la línea entre los nodos para añadir un color en ese punto.

La barra de propiedades tiene el tipo de relleno y sus ajustes: el color de un
relleno uniforme; para un degradado, el tipo, los colores inicial y final, el
ángulo y el relleno de borde.

## La herramienta Relleno de área {#area-fill}

La herramienta [[tool.area_fill]] rellena cualquier área cerrada, incluso una
formada por varios objetos superpuestos, creando un objeto nuevo con esa
área. Los objetos que la rodean quedan como están.

1. Elija la herramienta [[tool.area_fill]].
2. Elija el relleno y el contorno en la barra de propiedades: **Especificar**
   (el color y, para el contorno, el grosor, al lado), **Usar predeterminado**
   (el relleno y el contorno de los objetos nuevos; un color cuando es sin
   relleno, para que el área se vea) o **Ninguno**.
3. Haga clic dentro del área.

El nuevo objeto se coloca encima de la capa activa y queda seleccionado. Un
clic sobre un contorno, o en un área que nada encierra, no crea nada.

## Rellenos de malla {#mesh-fill}

Un relleno de malla es una cuadrícula de nodos sobre un objeto, cada nodo con
su propio color y transparencia; los colores fluyen suavemente entre los
nodos. Es adecuado para sombreados realistas.

1. Elija la herramienta [[tool.mesh_fill]] (<kbd>M</kbd>) y ajuste las filas y
   columnas en la barra de propiedades (2 por 2 de forma predeterminada, hasta
   50).
2. Haga clic en un objeto. Recibe una malla del color de su relleno.
3. Haga clic en un nodo para seleccionarlo y luego en un color de la paleta
   para colorearlo. Arrastre nodos para curvar el flujo de colores.
4. Haga doble clic dentro de la malla para añadir una fila o columna que pase
   por el nodo más cercano; seleccione un nodo y pulse <kbd>Delete</kbd> para
   quitar su fila y su columna.

La barra de propiedades también tiene **Suavizar color**, que funde los
colores con más suavidad, la **transparencia** del nodo seleccionado y
**Borrar malla**, que quita el relleno de malla. Cambiar las filas y columnas
de una malla conserva sus colores.

## Rellenos de patrón y de textura {#pattern-texture}

- Un relleno [[docker.fill_pattern]] repite un mosaico: dos colores, un patrón
  vectorial o un mapa de bits. Vea [Rellenos de patrón](patterns.md).
- Un relleno [[docker.fill_texture]] es una textura generada de aspecto
  natural: [[texture.clouds]], [[texture.marble]], [[texture.noise]] o
  [[texture.wood]], con sus colores y su escala.

## Varios rellenos en un objeto {#several-fills}

En la sección de relleno del panel [[docker.properties]], [[docker.add_fill]]
pinta otro relleno sobre el propio del objeto, empezando como una copia que
puede cambiar. Los rellenos adicionales aparecen como "Relleno 2", "Relleno 3",
etc., cada uno con su propio editor y un botón para quitarlo. La sección de
contorno puede añadir más contornos de la misma manera.

También se pueden pintar rellenos sobre los contornos; vea
[Ajustar contornos](../lines-shapes/outlines.md#outline-pen).
