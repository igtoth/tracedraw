# 颜色面板

[[docker.color]] 面板（<kbd>Shift+F11</kbd>）用于调配颜色，并将其应用到所选对象的填充或轮廓。[[tool.outline_color]] 工具（<kbd>Shift+F12</kbd>）也会打开它。可从 [[menu.window > menu.window.dockers]] 打开。

![颜色面板](../../img/color-color-docker.png)

## 选择颜色 {#choose}

1. 选择对象。面板从第一个对象的填充颜色开始。
2. 在以下某个视图中调配新颜色：
   - [[docker.color_viewers]]：在方块中单击或拖动以设置饱和度（从左到右）和亮度（从下到上），在旁边的色条中设置色相；
   - [[docker.color_sliders]]：[[docker.model]] 的每个分量一个滑块：RGB、CMYK（起初选用）、HSB、HSL、Lab、灰度或 YIQ；
   - [[docker.color_palettes]]：某个调色板的色样。

   顶部的三个按钮分别显示或隐藏各个视图；可以显示任意组合。你也可以输入 `#rrggbb` 值。
3. 单击面板底部的 [[docker.fill]] 或 [[docker.outline]] 应用新颜色。锁（[[docker.lock_colors]]）把填充和轮廓绑在一起，任一按钮都会把颜色同时应用到两者。"..." 按钮（[[docker.more_actions]]）展开其他操作：[[docker.no_fill]], [[docker.no_outline_action]], [[docker.swap_colors]], [[docker.add_to_document_palette]], [[docker.add_to_spot_colors]] 和 [[docker.add_to_color_styles]]。

旧颜色和新颜色并排显示。当新颜色无法用 CMYK 油墨准确印刷时，面板会显示警告。

## 取样颜色 {#sample}

单击 [[docker.eyedropper]]，再单击绘图上的任意位置以获取该处的颜色。然后单击 [[docker.use_sampled]] 将其设为新颜色。另请参阅 [滴管](eyedroppers.md)。

## 专色油墨 {#spot-inks}

在按钮下方，[[docker.spot_inks]] 列出绘图的专色油墨；请参阅 [专色](spot-colors.md)。
