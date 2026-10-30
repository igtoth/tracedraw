# Растровые эффекты

Растровые эффекты меняют пиксели: размывают, повышают резкость, корректируют
цвета, добавляют текстуры и художественные мазки. Они работают с растровыми
изображениями, а также с векторными объектами, которые тогда рисуются как
картинка, созданная эффектами, сохраняя при этом свою форму, заливку и абрис.

Эффекты хранятся отдельно от исходных пикселей: любой из них можно позже
изменить, скрыть, переставить или удалить, а удаление последнего возвращает
оригинал.

## Применение растрового эффекта {#apply}

1. Выделите один или несколько объектов.
2. Выберите группу в меню [[menu.effects]] и эффект.
3. Эффекты без параметров применяются сразу. Остальные открывают диалоговое
   окно с предпросмотром **до** и **после** для первого выделенного объекта:
   задайте значения и нажмите [[dialog.ok]]. Параметры расположены на
   панели справа; под ней **Сброс** возвращает все параметры к значениям по
   умолчанию, а две стрелки отменяют и повторяют изменения, сделанные в
   диалоговом окне.

![Диалоговое окно растрового эффекта](../../img/effects-bitmap-effects.png)

Изменение всего выделенного является одним шагом отмены. Когда выделено
растровое изображение, <kbd>Ctrl+B</kbd> открывает
[[fx.brightness_contrast_intensity]], <kbd>Ctrl+Shift+B</kbd>
[[fx.color_balance]], а <kbd>Ctrl+Shift+U</kbd> [[fx.hue_saturation_lightness]].

## Группы эффектов {#groups}

| Группа | Эффекты |
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

## Особые диалоговые окна {#special-dialogs}

- **[[fx.image_adjustments]]** открывает большое диалоговое окно: крупный
  предпросмотр, который можно масштабировать, прокручивать и поворачивать,
  показанный отдельно, рядом с оригиналом или с разделением; **Автокоррекция**
  и пипетки для точек белого и чёрного; ползунки температуры, оттенка,
  насыщенности, яркости, контраста, светов, теней и средних тонов с
  гистограммой; отмена и повтор внутри окна; снимки параметров для сравнения.
- **[[fx.contrast_enhancement]]** показывает гистограмму с маркерами входных и
  выходных уровней, пипетки для чёрного и белого и **Автокоррекцию**.
- **[[fx.tone_curve]]** редактирует кривую для каждого канала (общий,
  красный, зелёный, синий) в четырёх стилях (кривая, прямые, от руки, гамма)
  поверх гистограммы; кривые можно сохранять и загружать.
- У цветов картинки, например в [[fx.replace_colors]], есть пипетка:
  щёлкните предпросмотр, чтобы взять цвет под указателем.

## Правка эффектов объекта {#fx-section}

Раздел [[docker.fx]] панели [[docker.properties]] перечисляет эффекты
выделенного объекта, первый применённый вверху. В каждой строке есть флажок,
чтобы показать или скрыть эффект, **Изменить** (открывает его диалоговое
окно), **Выше**, **Ниже** и **Удалить**. **Добавить эффект** открывает группы
эффектов.

[[menu.effects > menu.effects.flatten]] делает эффекты постоянными:
растровое изображение сохраняет то, что показывает, а векторный объект
становится растровым изображением.

## Выбор эффектов {#effect-picker}

[[menu.effects > menu.effects.picker]] открывает окно, которое остаётся
открытым во время работы, со всеми эффектами в виде маленьких картинок.
Вводите текст в поле поиска, чтобы найти эффекты, или выберите группу или
**Недавние** в списке. Когда что-то выделено, щёлкните плитку, чтобы применить
эффект. Эффект можно также применить как [линзу](lens.md).

## Эффекты, выходящие за края {#inflate}

Размытие и похожие эффекты могут выходить за края растрового изображения.
Когда [[menu.bitmaps > menu.bitmaps.inflate > menu.bitmaps.inflate_auto]]
включено (как по умолчанию), изображение увеличивается, чтобы освободить для
них место. См.
[Передискретизация и выравнивание](../bitmaps/resample-and-straighten.md#inflate).

Чтобы сохранить растровое изображение с его эффектами в файл PNG, щёлкните его
правой кнопкой и выберите [[menu.bitmaps.save_bitmap_as]].
