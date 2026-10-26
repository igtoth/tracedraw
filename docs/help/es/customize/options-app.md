# Opciones de TraceDraw

[[menu.tools > menu.tools.options > menu.tools.options_app]] (<kbd>Ctrl+J</kbd>)
ajusta cómo funciona el programa. Elija una página de la lista de la
izquierda.

## General {#general}

![La página General de las opciones de TraceDraw](../../img/customize-options-app-general.png)

- [[options.on_startup]]: mostrar la pantalla de bienvenida, empezar un
  documento nuevo o abrir el último documento editado.
- [[options.show_new_document_dialog]] (activado): pedir los ajustes de un
  dibujo nuevo; vea [Crear un dibujo](../get-started/new-document.md).
- [[options.show_splash]] (activado).
- [[options.hardware_acceleration]] (activado): dibuja la ventana con la tarjeta gráfica elegida al lado ([[options.gpu_automatic]] al principio); desactivado, con el renderizador por software del equipo. Se aplica en el siguiente inicio. Si la ventana no se abre así, TraceDraw vuelve a iniciarse con el renderizador por software y luego con OpenGL, y conserva el que funcionó.
- [[options.undo_levels]]: cuántos pasos puede deshacer (150 de forma
  predeterminada).

## Visualización {#display}

![La página Visualización de las opciones de TraceDraw](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: mostrar los colores como se imprimirán.
- [[options.show_tooltips]] (activado), [[options.hide_bbox_curve_tools]] y
  [[options.node_tracking]] (desactivado: si se activa, la herramienta
  [[tool.pick]] puede arrastrar los nodos de una curva seleccionada).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (opción
  predeterminada) o [[options.wheel_scroll]].
- [[options.ruler_calibration]]: sostenga una regla junto a la pantalla y
  cambie la [[options.screen_resolution]] hasta que las marcas coincidan, para
  que el zoom al 100% muestre los tamaños reales. [[options.reset]] vuelve a
  96.

## Edición {#edit}

![La página Edición de las opciones de TraceDraw](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: el paso de <kbd>Ctrl</kbd> al girar y al
  dibujar (15 grados de forma predeterminada).
- [[options.drawing_precision]]: los decimales de la barra de propiedades y de
  la barra de estado (3 de forma predeterminada).
- [[options.focus_inactive]] (20% de forma predeterminada) y
  [[options.focus_double_click]]; vea [Modo de enfoque](../objects/focus-mode.md).

## Nodos y tiradores {#nodes-and-handles}

![La página Nodos y tiradores de las opciones de TraceDraw](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: pequeño, mediano o grande.
- [[options.node_shape]] para los nodos [[options.node_cusp]],
  [[options.node_smooth]] y [[options.node_symmetrical]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: el [[options.main_color]] de los nodos y tiradores
  seleccionados y el [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Marco de recorte {#clip-frame}

![La página Marco de recorte de las opciones de TraceDraw](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (opción
  predeterminada), [[options.auto_center_always]] o
  [[options.auto_center_never]].
- [[options.empty_frame_lines]] (activado).

Vea [Colocar objetos dentro de un marco](../objects/clip-frames.md).

## Ajuste {#snapping}

![La página Ajuste de las opciones de TraceDraw](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], la cuadrícula de
  línea de base y [[options.snap_to_pixels]].
- [[options.snapping_radius]] (10 píxeles de forma predeterminada).
- [[options.show_snap_marks]] y la [[options.snap_screen_tip]].
- [[options.snap_modes]]: los puntos a los que se ajustan los objetos, con
  [[options.select_all]] y [[options.deselect_all]].

Vea [Ajuste](../pages-layout/snapping.md).

## Guardar {#save}

![La página Guardar de las opciones de TraceDraw](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (activado) conserva el
  archivo anterior como `backup_of_<name>`, [[options.backup_to]] la
  [[options.same_folder]] o un lugar elegido.
- [[options.auto_backup]] (activado): [[options.backup_every]] 20
  [[options.minutes]], en una carpeta temporal o en un lugar elegido. Tras un
  bloqueo se ofrecen las copias; vea
  [Copia de seguridad automática y recuperación](../get-started/open-and-save.md#recovery).

## Texto {#text}

![La página Texto de las opciones de TraceDraw](../../img/customize-options-app-text.png)

La página tiene cinco pestañas.

- [[options.display]]: [[options.keyboard_text_increment]] (cuánto agrandan y
  reducen <kbd>Ctrl+8</kbd> y <kbd>Ctrl+2</kbd> el texto seleccionado, 1 punto),
  la [[options.default_font]] y el [[options.default_size]] del texto nuevo, y
  [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] y los [[options.hyphenation_patterns]]
  (integrados, o un diccionario de división de su idioma).
- [[docker.fonts]]: [[options.show_recent_fonts]] arriba de las listas de
  fuentes, el [[options.recent_font_count]] (5, hasta 20) y el
  [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  y [[options.add_to_autocorrect]], para que una palabra corregida en el
  corrector se corrija al escribir la próxima vez; y los
  [[options.dictionaries]] que usa, consulte
  [Diccionarios](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: lo que se corrige al escribir; consulte
  [Autocorrección](../text/writing-tools.md#autocorrect).
