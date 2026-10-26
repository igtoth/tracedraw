# TraceDraw 选项

[[menu.tools > menu.tools.options > menu.tools.options_app]]（<kbd>Ctrl+J</kbd>）设置程序的工作方式。从左侧列表中选择一个页面。

## 常规 {#general}

![TraceDraw 选项的常规页面](../../img/customize-options-app-general.png)

- [[options.on_startup]]：显示欢迎屏幕、新建文档，或打开上次编辑的文档。
- [[options.show_new_document_dialog]]（打开）：询问新绘图的设置；请参阅 [创建绘图](../get-started/new-document.md)。
- [[options.show_splash]]（打开）。
- [[options.hardware_acceleration]]（开）：用旁边选择的显卡绘制窗口（开始时为 [[options.gpu_automatic]]）；关闭时使用计算机的软件渲染器。下次启动时生效。如果窗口无法这样打开，TraceDraw 会改用软件渲染器、再改用 OpenGL 重新启动，并记住可用的那个。
- [[options.undo_levels]]：可以撤消的步骤数（起初为 150）。

## 显示 {#display}

![TraceDraw 选项的显示页面](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]：按印刷效果显示颜色。
- [[options.show_tooltips]]（打开）、[[options.hide_bbox_curve_tools]] 和 [[options.node_tracking]]（关闭：打开时，[[tool.pick]] 工具可以拖动所选曲线的节点）。
- [[options.full_screen_preview]]：[[options.show_page_border]]。
- [[options.wheel_default_action]]：[[options.wheel_zoom]]（起初选用）或 [[options.wheel_scroll]]。
- [[options.ruler_calibration]]：将一把尺子贴在屏幕上，更改 [[options.screen_resolution]] 直到刻度吻合，使 100% 缩放显示真实尺寸。[[options.reset]] 恢复为 96。

## 编辑 {#edit}

![TraceDraw 选项的编辑页面](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]：旋转和绘图时 <kbd>Ctrl</kbd> 的角度步长（起初为 15 度）。
- [[options.drawing_precision]]：属性栏和状态栏上的小数位数（起初为 3）。
- [[options.focus_inactive]]（起初为 20%）和 [[options.focus_double_click]]；请参阅 [焦点模式](../objects/focus-mode.md)。

## 节点和控制柄 {#nodes-and-handles}

![TraceDraw 选项的节点和控制柄页面](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]：小、中或大。
- [[options.node_cusp]]、[[options.node_smooth]] 和 [[options.node_symmetrical]] 节点的 [[options.node_shape]]。
- [[options.show_curve_direction]]。
- [[options.corner_widgets]]。
- [[options.colors]]：所选节点和控制柄的 [[options.main_color]] 以及 [[options.secondary_color]]。
- [[options.unselected_nodes_filled]]。

## 剪裁框 {#clip-frame}

![TraceDraw 选项的剪裁框页面](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]：[[options.auto_center_outside]]（起初选用）、[[options.auto_center_always]] 或 [[options.auto_center_never]]。
- [[options.empty_frame_lines]]（打开）。

请参阅 [将对象置于框内](../objects/clip-frames.md)。

## 贴齐 {#snapping}

![TraceDraw 选项的贴齐页面](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]]、[[options.snap_to_page]]、[[options.snap_to_guidelines]]、[[options.snap_to_grid]]、基线网格和 [[options.snap_to_pixels]]。
- [[options.snapping_radius]]（起初为 10 像素）。
- [[options.show_snap_marks]] 和 [[options.snap_screen_tip]]。
- [[options.snap_modes]]：对象可贴齐的点，带有 [[options.select_all]] 和 [[options.deselect_all]]。

请参阅 [贴齐](../pages-layout/snapping.md)。

## 保存 {#save}

![TraceDraw 选项的保存页面](../../img/customize-options-app-save.png)

- [[options.backup]]：[[options.backup_before_save]]（打开）将之前的文件保留为 `backup_of_<name>`，[[options.backup_to]] [[options.same_folder]] 或所选位置。
- [[options.auto_backup]]（打开）：[[options.backup_every]] 20 [[options.minutes]]，保存到临时文件夹或所选位置。程序崩溃后会提供这些备份；请参阅 [自动备份和恢复](../get-started/open-and-save.md#recovery)。

## 文本 {#text}

![TraceDraw 选项的文本页面](../../img/customize-options-app-text.png)

此页面有五个选项卡。

- [[options.display]]：[[options.keyboard_text_increment]]（<kbd>Ctrl+8</kbd> 和
  <kbd>Ctrl+2</kbd> 放大和缩小所选文本的幅度，1 点）、新文本的
  [[options.text_units]] [[options.default_font]] 和 [[options.default_size]]，以及
  [[menu.text.show_non_printing]]。
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]：[[menu.text.show_text_frames]]、
  [[menu.text.use_hyphenation]] 和 [[options.hyphenation_patterns]]
  （内置，或您所用语言的断字词典）。
- [[docker.fonts]]：在字体列表顶部 [[options.show_recent_fonts]]、
  [[options.recent_font_count]]（5，最多 20）以及
  [[options.font_sample_size]]。
- [[options.spelling]]：[[writing.ignore_digits]]、[[writing.ignore_capitals]]
  和 [[options.add_to_autocorrect]]，这样在拼写检查中更正的单词下次键入时会被自动更正；以及它使用的
  [[options.dictionaries]]，请参阅[词典](../text/writing-tools.md#dictionaries)。
- [[dialog.autocorrect]]：键入时更正的内容；请参阅
  [自动更正](../text/writing-tools.md#autocorrect)。
