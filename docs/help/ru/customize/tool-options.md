# Параметры инструментов

> *Машинный перевод: эта страница может отставать от английской версии.*

[[menu.tools > menu.tools.options > menu.tools.options_tools]] задаёт, как
ведут себя инструменты и какие новые объекты они рисуют. Большинство этих
значений есть и на панели свойств каждого инструмента.

## Выбор {#pick}

![Параметры инструмента выбора](../../img/customize-tool-options-pick.png)

- [[options.crosshair_cursor]]: указатель в виде перекрестия.
- [[options.treat_all_filled]] (включено): щелчок внутри объекта без заливки
  выделяет его.
- [[options.marquee_touching]] (выключено): рамка выделения выделяет и
  объекты, которых она только касается; тогда <kbd>Alt</kbd> выделяет только
  объекты, целиком находящиеся внутри.

## Ластик {#eraser}

![Параметры инструмента «Ластик»](../../img/customize-tool-options-eraser.png)

[[options.eraser_width]] (по умолчанию 6,35 мм), а также
[[options.eraser_round]] или [[options.eraser_square]].

## Масштаб и панорамирование {#zoom-pan}

![Параметры масштаба и панорамирования](../../img/customize-tool-options-zoom-pan.png)

- [[options.zoom_right_button]]: [[options.zoom_out]] (по умолчанию) или
  [[options.context_menu]].
- [[options.wheel_action]]: [[options.wheel_zoom]] или
  [[options.wheel_scroll]].
- [[options.zoom_relative_1to1]] (включено): масштаб 100% равен реальному размеру при
  откалиброванном разрешении экрана; выключено: 96 пикселей на дюйм.
- [[options.wheel_center_mouse]] (выключено): масштабирование колесом переносит точку
  под указателем в центр вида вместе с указателем.
- [[options.pan_right_button]] [[options.zoom_out]] или [[options.context_menu]] (сначала).

## Свободная рука/Безье {#curves}

![Параметры инструментов «Свободная рука/Безье»](../../img/customize-tool-options-curves.png)

- [[options.freehand_smoothing]]: от 0 до 100 (по умолчанию 50).
- [[options.join_within]]: насколько близко к конечному узлу должна начаться
  линия, чтобы продолжить кривую (по умолчанию 5 пикселей экрана). См.
  [Продолжение кривой](../lines-shapes/lines.md#continue).

- [[options.pen_preview]] и [[options.pen_auto_add_delete]] (оба включены)
  для инструмента [[tool.pen]]; см. [Перо](../lines-shapes/lines.md#pen).
- [[options.pen_tablet]]: [[options.pen_pressure]] (включено); см.
  [Рисование на графическом планшете](../lines-shapes/pen-pressure.md).

## Распознавание фигур {#shape-recognition}

![Параметры инструмента «Распознавание фигур»](../../img/customize-tool-options-shape-recognition.png)

[[recognize.recognition]] и [[recognize.smoothing]] инструмента
«Распознавание фигур», от выключено до максимума, как на его панели
свойств.

## Прямоугольник {#rectangle}

![Параметры инструмента «Прямоугольник»](../../img/customize-tool-options-rectangle.png)

[[options.rectangle_corners]]: [[toolbar.corner_round]],
[[toolbar.corner_scallop]] или [[toolbar.corner_chamfer]], четыре размера
углов и [[toolbar.corners_together]]. [[options.scale_corners]]:
[[toolbar.relative_corners]].

## Эллипс {#ellipse}

![Параметры инструмента «Эллипс»](../../img/customize-tool-options-ellipse.png)

[[options.ellipse]], [[options.pie]] или [[options.arc]], с
[[options.starting_angle]] и [[options.ending_angle]].

## Многоугольник {#polygon}

![Параметры инструмента «Многоугольник»](../../img/customize-tool-options-polygon.png)

[[options.number_of_points]] и [[options.sharpness]].

## Миллиметровка {#graph-paper}

![Параметры инструмента «Миллиметровка»](../../img/customize-tool-options-graph-paper.png)

[[options.rows]] и [[options.columns]] сетки.

## Спираль {#spiral}

![Параметры инструмента «Спираль»](../../img/customize-tool-options-spiral.png)

[[options.revolutions]], а также [[options.symmetrical_spiral]] или
[[options.logarithmic_spiral]].

## Таблица {#table}

![Параметры инструмента «Таблица»](../../img/customize-tool-options-table.png)

[[options.rows]] и [[options.columns]] новых таблиц.

## Размер {#dimension}

![Параметры инструмента «Размер»](../../img/customize-tool-options-dimension.png)

Как выглядит подпись новых размеров: [[dimension.precision]],
[[dimension.units]], [[dimension.show_units]], [[dimension.prefix]] и
[[dimension.suffix]]. Уже нарисованные размеры сохраняют свою подпись;
выделите размер и измените её на панели свойств.

## Соединительная линия {#connector}

![Параметры инструмента «Соединительная линия»](../../img/customize-tool-options-connector.png)

[[connector.radius]] новых скруглённых соединительных линий (сначала 3 мм).

## Заливка сетки {#mesh-fill}

![Параметры инструмента «Заливка сетки»](../../img/customize-tool-options-mesh-fill.png)

[[options.columns]] и [[options.rows]] сетки, с которой начинается новая
заливка сетки (сначала 2 на 2).
