# Efectos de mapa de bits

Los efectos de mapa de bits cambian píxeles: desenfocan, enfocan, ajustan
colores, añaden texturas y trazos artísticos. Funcionan con mapas de bits y
también con objetos vectoriales, que entonces se dibujan como la imagen que
crean los efectos, aunque conservan su propia forma, relleno y contorno.

Los efectos se mantienen separados de los píxeles originales: puede editar,
ocultar, reordenar o quitar cualquiera de ellos más tarde, y al quitar el
último vuelve el original.

## Para aplicar un efecto de mapa de bits {#apply}

1. Seleccione uno o varios objetos.
2. Elija un grupo en el menú [[menu.effects]] y un efecto.
3. Los efectos sin ajustes se aplican de inmediato. Los demás abren un cuadro
   de diálogo con vistas previas de **antes** y **después** del primer objeto
   seleccionado: ajuste los valores y haga clic en [[dialog.ok]].
   Los ajustes están en un panel a la derecha; debajo de él, **Restablecer**
   devuelve todos los ajustes a su valor predeterminado y las dos flechas
   deshacen y rehacen los cambios hechos en el cuadro de diálogo.

![Un cuadro de diálogo de efecto de mapa de bits](../../img/effects-bitmap-effects.png)

El cambio en toda la selección es un solo paso de deshacer. Con un mapa de
bits seleccionado, <kbd>Ctrl+B</kbd> abre
[[fx.brightness_contrast_intensity]], <kbd>Ctrl+Shift+B</kbd>
[[fx.color_balance]] y <kbd>Ctrl+Shift+U</kbd> [[fx.hue_saturation_lightness]].

## Los grupos de efectos {#groups}

| Grupo | Efectos |
|---|---|
| [[menu.effects.3d]] | [[fx.cylinder]], [[fx.emboss]], [[fx.page_curl]], [[fx.pinch_punch]], [[fx.sphere]] |
| [[menu.effects.adjust]] | [[fx.auto_adjust]], [[fx.image_adjustments]], [[fx.contrast_enhancement]], [[fx.local_equalization]], [[fx.target_balance]], [[fx.tone_curve]], [[fx.brightness_contrast_intensity]], [[fx.color_balance]], [[fx.gamma]], [[fx.hue_saturation_lightness]], [[fx.selective_color]], [[fx.replace_colors]], [[fx.desaturate]], [[fx.channel_mixer]] |
| [[menu.effects.art_strokes]] | [[fx.charcoal]], [[fx.conte_crayon]], [[fx.crayon]], [[fx.cubist]], [[fx.impressionist]], [[fx.palette_knife]], [[fx.pastels]], [[fx.pen_ink]], [[fx.pointillist]], [[fx.scraperboard]], [[fx.sketch_pad]], [[fx.watercolor]], [[fx.felt_marker]], [[fx.paper_grain]] |
| [[menu.effects.blur]] | [[fx.directional_smooth]], [[fx.gaussian_blur]], [[fx.jagged_edges]], [[fx.low_pass]], [[fx.motion_blur]], [[fx.radial_blur]], [[fx.smart_blur]], [[fx.smooth]], [[fx.soften]], [[fx.zoom]] |
| [[menu.effects.camera]] | [[fx.colorize]], [[fx.diffuse]], [[fx.photo_filter]], [[fx.sepia_toning]], [[fx.vintage_photo]] |
| [[menu.effects.color_transform]] | [[fx.bit_planes]], [[fx.halftone]], [[fx.psychedelic]], [[fx.solarize]] |
| [[menu.effects.contour_group]] | [[fx.edge_detect]], [[fx.find_edges]], [[fx.trace_contour]] |
| [[menu.effects.correction]] | [[fx.dust_and_scratch]] |
| [[menu.effects.creative]] | [[fx.crystallize]], [[fx.fabric]], [[fx.frame]], [[fx.glass_tiles]], [[fx.mosaic]], [[fx.scatter]], [[fx.tinted_glass]], [[fx.stained_glass]], [[fx.vignette]], [[fx.vortex]] |
| [[menu.effects.custom]] | [[fx.bump_map]] |
| [[menu.effects.distort]] | [[fx.blocks]], [[fx.displace]], [[fx.mesh_warp]], [[fx.offset]], [[fx.pixelate]], [[fx.ripple]], [[fx.swirl]], [[fx.tile]], [[fx.wet_paint]], [[fx.whirlpool]], [[fx.wind]] |
| [[menu.effects.noise]] | [[fx.add_noise]], [[fx.maximum]], [[fx.median]], [[fx.minimum]], [[fx.remove_moire]], [[fx.remove_noise]] |
| [[menu.effects.sharpen]] | [[fx.adaptive_unsharp]], [[fx.directional_sharpen]], [[fx.high_pass]], [[fx.sharpen]], [[fx.unsharp_mask]] |
| [[menu.effects.texture]] | [[fx.cobblestone]], [[fx.wrinkles]], [[fx.etching]], [[fx.plastic]], [[fx.relief_sculpture]], [[fx.stone]] |
| [[menu.effects.transform]] | [[fx.deinterlace]], [[fx.invert_colors]], [[fx.posterize]] |

## Cuadros de diálogo especiales {#special-dialogs}

- **[[fx.image_adjustments]]** abre un cuadro de diálogo más grande: una vista
  previa amplia que puede ampliar, desplazar y girar, mostrada sola, en
  paralelo o dividida; **Ajuste automático** y selectores para los puntos
  blanco y negro; deslizadores de temperatura, matiz, saturación, brillo,
  contraste, luces, sombras y tonos medios, con un histograma; deshacer y
  rehacer dentro del cuadro de diálogo; e instantáneas de ajustes para
  comparar.
- **[[fx.contrast_enhancement]]** muestra el histograma con marcadores que se
  arrastran para los niveles de entrada y salida, cuentagotas para el negro y
  el blanco, y **Ajuste automático**.
- **[[fx.tone_curve]]** edita una curva por canal (maestro, rojo, verde, azul)
  en cuatro estilos (curva, recta, a mano alzada, gamma), sobre el histograma;
  las curvas se pueden guardar y cargar.
- Los colores de la imagen, como en [[fx.replace_colors]], tienen un
  cuentagotas: haga clic en una vista previa para tomar el color que hay bajo
  el puntero.

## Editar los efectos de un objeto {#fx-section}

La sección [[docker.fx]] del panel [[docker.properties]] enumera los efectos
del objeto seleccionado, con el primero aplicado arriba. Cada fila tiene una
casilla para mostrar u ocultar el efecto, **Editar** (abre su cuadro de
diálogo), **Subir**, **Bajar** y **Eliminar**. **Añadir efecto** abre los
grupos de efectos.

[[menu.effects > menu.effects.flatten]] hace permanentes los efectos: un mapa
de bits conserva lo que muestra, y un objeto vectorial se convierte en mapa de
bits.

## El Selector de efectos {#effect-picker}

[[menu.effects > menu.effects.picker]] abre una ventana que permanece abierta
mientras trabaja, con cada efecto como una pequeña imagen. Escriba en el campo
de búsqueda para encontrar efectos, o elija un grupo o **Recientes** en la
lista. Con algo seleccionado, haga clic en un mosaico para aplicar el efecto.
También puede aplicar un efecto como [lente](lens.md).

## Efectos que se extienden más allá de los bordes {#inflate}

Los desenfoques y efectos similares pueden llegar más allá de los bordes de un
mapa de bits. Con
[[menu.bitmaps > menu.bitmaps.inflate > menu.bitmaps.inflate_auto]] activado
(como de forma predeterminada), el mapa de bits crece para dejarles sitio. Vea
[Remuestrear y enderezar](../bitmaps/resample-and-straighten.md#inflate).

Para guardar un mapa de bits con sus efectos como archivo PNG, haga clic en él
con el botón derecho y elija [[menu.bitmaps.save_bitmap_as]].
