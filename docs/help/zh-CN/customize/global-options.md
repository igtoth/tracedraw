# 全局选项

[[menu.tools > menu.tools.options > menu.tools.options_global]] 包含适用于整个程序的设置：语言、打印和导出的默认值、描摹、消息以及文件的存放位置。

## 常规 {#general}

![全局选项的常规页面](../../img/customize-global-options-general.png)

- [[options.language]]：界面的语言。本帮助以相同的语言打开。
- [[options.check_updates]]（打开）：请参阅 [检查更新](../welcome/about-and-updates.md#updates)。
- [[options.agents]]：[[options.mcp_window]] 允许此计算机上的 AI 助手编辑打开的绘图；请参阅 [脚本](scripts.md#mcp)。
- [[options.reset_all]]：请参阅 [重置设置](reset-settings.md)。

## 打印 {#printing}

![全局选项的打印页面](../../img/customize-global-options-printing.png)

[[options.print_starts_with]]：[[dialog.fit_to_page]]、[[dialog.separations]]、[[dialog.print_bleed]]、[[dialog.crop_marks]]、[[dialog.registration_marks]]、[[print.color_bars]]、[[print.densitometer]]、[[print.page_info_mark]]（均关闭）、[[print.sheet]]、[[print.landscape]] 和 [[print.gutter]]（5 mm）。[[options.print_remember]] 保留上次打印的选择。在 [[options.print_checks]] 下，[[options.print_preflight]]（打开）。请参阅 [打印绘图](../printing/print.md)。

## 文件格式 {#file-formats}

![全局选项的文件格式页面](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]]：[[dialog.format]]（SVG）、位图格式的 [[dialog.resolution]]（300 dpi）、JPEG 和 WebP 的 [[dialog.quality]]（90）、[[dialog.transparent_background]]（打开）、[[dialog.pdf_standard]]、[[dialog.output_condition]] 和 [[dialog.bleed]]。[[options.export_remember]] 保留上次导出的选择。
- [[options.import_section]]：[[options.import_place]] [[options.import_as_in_file]]（起初选用）或 [[options.import_view_center]]。
- [[options.cdr_save_section]]：新绘图另存为 `.cdr` 时使用的 [[dialog.cdr_version]]。

## 位图描摹 {#bitmap-tracing}

![全局选项的位图描摹页面](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]] 预设之一（起初为剪贴画）。[[options.trace_starts_with]] [[dialog.remove_background]]（打开）和 [[dialog.delete_original]]（关闭）。请参阅 [描摹位图](../bitmaps/tracing.md)。

## 警告 {#warnings}

![全局选项的警告页面](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]]、[[options.warn_missing_fonts]] 和 [[options.warn_shelf_remove]]（均打开）。每条消息还有一个用于停止显示它的复选框；在此处勾选该选项可再次看到它。

## 文件位置 {#file-locations}

![全局选项的文件位置页面](../../img/customize-global-options-file-locations.png)

显示首选项和自动备份的存放位置。如果程序旁有 `tracedraw-settings` 文件夹（便携版），首选项就保存在其中，否则保存在系统存放程序设置的位置；浏览器版本将其保存在浏览器中。
