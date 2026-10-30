# Paletas de colores

La paleta de colores de la parte inferior de la ventana es la forma más rápida
de colorear objetos. Debajo de ella, la **paleta del documento** reúne los
colores que usa su dibujo. Puede abrir más paletas, crear las suyas y
editarlas.

## Usar la paleta {#using}

| Acción | Resultado |
|---|---|
| Hacer clic en un color | rellena la selección (sin nada seleccionado, fija el relleno de los objetos nuevos) |
| Hacer clic con el botón derecho en un color | colorea el contorno de la selección |
| <kbd>Ctrl</kbd>+clic en un color | mezcla una décima parte de él en el relleno de la selección |
| Mantener pulsado un color | muestra tonos más claros, más oscuros, más apagados y más vivos entre los que elegir |
| Arrastrar un color sobre un objeto | rellena el objeto, o colorea su contorno si se suelta sobre el borde |
| Hacer clic en la casilla tachada | quita el relleno (botón derecho: el contorno) |

Las flechas de los extremos desplazan la paleta, y el botón del extremo
derecho muestra todos los colores a la vez. Pase el puntero sobre un color
para ver su nombre y sus valores. El botón de menú de la paleta, en el extremo
izquierdo, abre el menú de la paleta.

La paleta se puede mover a otro borde de la ventana o dejarse flotante:
arrástrela por su asa punteada.

En cada lado puede quedar en el borde de la ventana o junto al dibujo, entre el dibujo y los paneles o la caja de herramientas de ese lado. Mientras la arrastra, una banda azul muestra dónde quedará; haga clic derecho en su asa para elegir un lugar de una lista.

## La paleta del documento {#document-palette}

Cada color que aplica a un objeto se une a la paleta del documento, que se
guarda con el dibujo. Su cuentagotas añade el color que hay bajo el puntero, y
su menú tiene **Actualizar automáticamente** (para que deje de añadir colores
por sí sola), **Añadir desde la selección**, **Añadir desde el documento**,
**Eliminar color** y **Restablecer paleta** (conserva solo los colores que el
dibujo aún usa). También puede arrastrar colores u objetos sobre ella. Para
mostrarla u ocultarla, vea
[Opciones de personalización](../customize/customization.md#color-palette).

## El panel Paletas {#palettes-docker}

El panel [[docker.palettes]] enumera las paletas que tiene, con el número de
colores de cada una. Ábralo con
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]].

![El panel Paletas](../../img/color-palettes.png)

- Marque una paleta para mostrarla bajo la ventana de dibujo.
- [[docker.edit]] la abre en el editor de paletas.
- [[menu.window.palette_open]] abre un archivo de paleta (`.tdpal`, `.gpl`,
  `.ase` o `.aco`).
- [[menu.window.palette_from_document]] y
  [[menu.window.palette_from_selection]] crean una paleta con los colores
  usados en el dibujo o en la selección.
- Los colores de la paleta del documento aparecen abajo; haga clic en uno para
  rellenar la selección, o con el botón derecho para colorear el contorno.

## Editar una paleta {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]] abre
el editor de paletas.

![El Editor de paletas](../../img/color-palettes-palette-editor.png)

1. Elija la [[dialog.palette]] que desea editar, o haga clic en
   [[dialog.new_palette]] para empezar una.
2. Escriba su [[dialog.name]].
3. Haga clic en un color para seleccionarlo; [[dialog.add_color]] añade el
   color actual y [[dialog.remove_color]] quita el seleccionado.
4. Haga clic en [[dialog.save_palette]] para guardarla como archivo.
5. Haga clic en [[dialog.close]].

Las paletas integradas no se pueden editar; cree una paleta nueva o edite una
copia.
