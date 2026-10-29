# Параметры TraceDraw

[[menu.tools > menu.tools.options > menu.tools.options_app]] (<kbd>Ctrl+J</kbd>)
задаёт, как работает программа. Выберите страницу в списке слева.

## Общие {#general}

![Страница «Общие» параметров TraceDraw](../../img/customize-options-app-general.png)

- [[options.on_startup]]: показать экран приветствия, начать новый документ
  или открыть последний редактировавшийся документ.
- [[options.show_new_document_dialog]] (включено): спрашивать параметры
  нового рисунка; см. [Создание рисунка](../get-started/new-document.md).
- [[options.show_splash]] (включено).
- [[options.hardware_acceleration]] (вкл.): окно рисуется видеокартой, выбранной рядом (сначала [[options.gpu_automatic]]); выкл. — программным рендерером компьютера. Действует со следующего запуска. Если окно так не открывается, TraceDraw запускается снова с программным рендерером, затем с OpenGL, и запоминает сработавший.
- [[options.undo_levels]]: сколько шагов можно отменить (по умолчанию 150).

## Отображение {#display}

![Страница «Отображение» параметров TraceDraw](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: показывать цвета такими, какими они будут
  напечатаны.
- [[options.show_tooltips]] (включено), [[options.hide_bbox_curve_tools]] и
  [[options.node_tracking]] (выключено: если включить, инструмент
  [[tool.pick]] может перетаскивать узлы выделенной кривой).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (по умолчанию) или
  [[options.wheel_scroll]].
- [[options.open_view]]: [[options.open_view_page]] (по умолчанию),
  [[options.open_view_objects]], [[options.open_view_width]],
  [[options.open_view_height]], [[options.open_view_actual]] или
  [[options.open_view_saved]] (страница, место и масштаб, запоминаемые при
  каждом сохранении файла TraceDraw).
- [[options.ruler_calibration]]: приложите линейку к экрану и меняйте
  [[options.screen_resolution]], пока деления не совпадут, чтобы масштаб 100%
  показывал реальные размеры. [[options.reset]] возвращает 96.

## Правка {#edit}

![Страница «Правка» параметров TraceDraw](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: шаг <kbd>Ctrl</kbd> при повороте и рисовании
  (по умолчанию 15 градусов).
- [[options.drawing_precision]]: число знаков после запятой на панели свойств
  и в строке состояния (по умолчанию 3).
- [[options.focus_inactive]] (по умолчанию 20%) и
  [[options.focus_double_click]]; см. [Режим фокуса](../objects/focus-mode.md).

## Узлы и маркеры {#nodes-and-handles}

![Страница «Узлы и маркеры» параметров TraceDraw](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: маленький, средний или большой.
- [[options.node_shape]] для узлов [[options.node_cusp]],
  [[options.node_smooth]] и [[options.node_symmetrical]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: [[options.main_color]] выделенных узлов и маркеров и
  [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Рамка обрезки {#clip-frame}

![Страница «Рамка обрезки» параметров TraceDraw](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (по умолчанию),
  [[options.auto_center_always]] или [[options.auto_center_never]].
- [[options.empty_frame_lines]] (включено).

См. [Размещение объектов внутри рамки](../objects/clip-frames.md).

## Привязка {#snapping}

![Страница «Привязка» параметров TraceDraw](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], сетка базовых
  линий и [[options.snap_to_pixels]].
- [[options.snapping_radius]] (по умолчанию 10 пикселей).
- [[options.show_snap_marks]] и [[options.snap_screen_tip]].
- [[options.snap_modes]]: точки, к которым привязываются объекты, с
  [[options.select_all]] и [[options.deselect_all]].

См. [Привязка](../pages-layout/snapping.md).

## Сохранение {#save}

![Страница «Сохранение» параметров TraceDraw](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (включено) сохраняет
  предыдущий файл как `backup_of_<name>`, [[options.backup_to]]
  [[options.same_folder]] или в выбранное место.
- [[options.auto_backup]] (включено): [[options.backup_every]] 20
  [[options.minutes]], во временную папку или в выбранное место. После сбоя
  предлагаются резервные копии; см.
  [Автоматическое резервное копирование и восстановление](../get-started/open-and-save.md#recovery).

## Текст {#text}

![Страница «Текст» параметров TraceDraw](../../img/customize-options-app-text.png)

На странице пять вкладок.

- [[options.display]]: [[options.keyboard_text_increment]] (на сколько
  <kbd>Ctrl+8</kbd> и <kbd>Ctrl+2</kbd> увеличивают и уменьшают выделенный
  текст, 1 пункт), [[options.default_font]] и [[options.default_size]] нового
  текста, а также [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] и [[options.hyphenation_patterns]]
  (встроенные или словарь переносов для вашего языка).
- [[docker.fonts]]: [[options.show_recent_fonts]] вверху списков шрифтов,
  [[options.recent_font_count]] (5, до 20) и
  [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  и [[options.add_to_autocorrect]], чтобы слово, исправленное при проверке
  орфографии, в следующий раз исправлялось при вводе; а также
  [[options.dictionaries]], которые она использует, см.
  [Словари](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: что исправляется при вводе; см.
  [Автозамена](../text/writing-tools.md#autocorrect).
