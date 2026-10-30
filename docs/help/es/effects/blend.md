# Mezclas

Una mezcla dibuja una serie de pasos intermedios entre dos objetos, cambiando
la forma, el tamaño y el color de uno a otro. Úsela para sombreados suaves,
patrones repetidos o una forma que se transforma en otra.

## Para mezclar dos objetos {#create}

Con la herramienta:

1. Elija la herramienta [[tool.blend]] en el menú desplegable Efectos.
2. Arrastre desde el objeto inicial hasta el objeto final.

Con el panel:

1. Seleccione dos objetos.
2. Abra el panel [[docker.blend]] con [[menu.effects > menu.effects.blend]]
   (<kbd>Ctrl+F8</kbd>).
3. Ajuste las opciones y haga clic en [[docker.apply]].

![El panel Mezcla](../../img/effects-blend.png)

Ambos objetos siguen siendo objetos del dibujo: mueva, cambie de tamaño o de
color cualquiera de ellos y los pasos lo siguen. Al eliminar el objeto final
se quita la mezcla.

## Ajustes de la mezcla {#settings}

Con una mezcla seleccionada, cada cambio en la barra de propiedades o en el
panel se aplica de inmediato.

- [[docker.blend_by_steps]]: el número de pasos (5 de forma predeterminada,
  hasta 999).
- **Espaciado**, para una mezcla sobre un trayecto: la distancia entre pasos;
  el número de pasos se deduce de la longitud del trayecto.
- **Rotación**: gira los pasos progresivamente; **Bucle** los gira en cambio
  alrededor del punto medio entre los dos objetos.
- **Aceleración**: el deslizador de objetos hace que los pasos se agolpen
  hacia un extremo; el deslizador de color hace lo mismo con los colores.
  [[docker.blend_apply_sizing]] hace que la aceleración de objetos cambie
  también los tamaños, y [[docker.blend_link_accel]] mantiene juntos los dos
  deslizadores. **Restablecer** devuelve ambos a un reparto uniforme.
- [[docker.blend_color_blend]]: [[docker.blend_direct]] (directamente de un
  color al otro), [[docker.blend_clockwise]] o
  [[docker.blend_counterclockwise]] (alrededor de la rueda de colores).

La barra de propiedades de la herramienta tiene los pasos, la rotación y las
dos aceleraciones.

## Mezclar a lo largo de un trayecto {#path}

1. Seleccione la mezcla.
2. Haga clic en **Nuevo trayecto** en el panel [[docker.blend]] y haga clic en
   una curva.

Los pasos siguen la curva. **Girar todos los objetos** orienta cada paso a lo
largo de la curva, y [[docker.blend_full_path]] coloca el primer y el último
paso en los extremos de la curva.

## Quitar o separar una mezcla {#clear}

**Borrar mezcla** quita los pasos y deja ambos objetos. **Separar** convierte
los pasos en objetos independientes.
Los pasos salen agrupados junto al objeto. [[menu.object > menu.object.break_blend_apart]] (<kbd>Ctrl+K</kbd>, también en
el menú del botón derecho) hace lo mismo.
