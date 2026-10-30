# Transparencia

La transparencia deja ver a través de un objeto lo que hay debajo. Puede ser
igual en todo el objeto, cambiar a lo largo de él como un degradado o seguir
un patrón o una textura.

## Aplicar transparencia {#apply}

- Con la herramienta [[tool.transparency]]: haga clic en un objeto y elija un
  tipo de transparencia en la barra de propiedades, o arrastre sobre el objeto
  para obtener una transparencia degradada de opaco (donde pulsa) a
  transparente (donde suelta). Los nodos del degradado aparecen entonces en el objeto, como con la herramienta [[tool.interactive_fill]]: arrastre el inicio, el final o el nodo redondo de inclinación; arrastre la barra entre dos nodos (el punto medio) para mover dónde la mezcla llega a la mitad; haga doble clic en la línea para añadir un nodo y en un nodo para quitarlo.
- En la sección [[docker.transparency]] del panel [[docker.properties]].

## Ajustes {#settings}

- El tipo: [[transparency.none]], [[transparency.uniform]] (opción
  predeterminada), [[transparency.fountain]] (lineal, radial, cónica, cuadrada
  o con forma, como los rellenos degradados), [[transparency.pattern]] o
  [[transparency.texture]].
- La cantidad: de 0 a 100% (50% de forma predeterminada). En un degradado,
  las paradas son tonos de gris: el blanco es opaco y el negro totalmente
  transparente.
- [[transparency.merge_mode]]: cómo se mezclan los colores con lo que hay
  debajo. Normal (opción predeterminada) simplemente deja ver; los demás, como
  Sumar, Restar, Diferencia, Multiplicar, Dividir, Si más claro, Si más
  oscuro, Tono, Saturación, Color e Invertir, combinan los colores de otras
  maneras.
- Si se aplica al [[transparency.target_fill]], al
  [[transparency.target_outline]] o a [[transparency.target_all]].
- **Congelar**: mantiene lo que se ve a través tal como está ahora, aunque se
  muevan los objetos de debajo.

Para quitarla, haga clic en [[toolbar.clear_transparency]] al final de la barra de propiedades (o elija [[transparency.none]] como tipo).

Para copiar la transparencia de otro objeto, use
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
Para zonas transparentes definidas por el brillo de un objeto, vea
[Máscaras de opacidad](../objects/clip-frames.md#opacity-mask).
