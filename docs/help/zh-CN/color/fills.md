# 填充

填充为闭合对象的内部着色。TraceDraw 有六种填充：[[docker.fill_none]]、[[docker.fill_uniform]]（一种纯色）、[[docker.fill_fountain]]（相互过渡的多种颜色）、[[docker.fill_pattern]]、[[docker.fill_texture]] 和 [[docker.fill_mesh]]（在网格节点之间流动的颜色）。

## 填充对象的方法 {#ways}

- 单击调色板中的颜色，为所选对象赋予均匀填充。
- 使用 [[docker.properties]] 面板（<kbd>F11</kbd>）的 [[docker.fill]] 部分：用一排图片选择填充类型，然后设置其选项。
- 使用 [[tool.interactive_fill]] 工具（<kbd>G</kbd>）直接在对象上设置渐变。
- 使用 [[tool.area_fill]] 工具填充由多个对象围成的区域。
- 使用 [[tool.mesh_fill]] 工具（<kbd>M</kbd>）进行网状填充。

未选择任何对象时，你选择的填充将成为新对象的填充。

## 均匀填充 {#uniform}

均匀填充是一种颜色，可以使用任何颜色模型。从调色板、[[docker.color]] 面板（请参阅 [颜色面板](color-docker.md)）或 [[docker.properties]] 面板的填充部分选择它。

## 渐变填充 {#gradient}

渐变将两种或更多颜色过渡混合。其设置位于 [[docker.properties]] 面板的填充部分：

- 类型：[[fill.linear]]（沿一条直线）、[[fill.radial]]（从中心向外）、[[fill.conical]]（围绕中心）、[[fill.square]]（逐渐扩大的方形）或 [[fill.shaped]]（沿对象轮廓从边缘向内）。
- [[docker.gradient_presets]]：可作为起点的现成渐变。
- **色标**：颜色及其在渐变中的位置，2 到 64 个。选择一个色标可更改其颜色或位置；[[docker.add_stop]] 和 [[docker.remove_stop]] 添加和删除色标。[[docker.import_gradient]] 从 `.ggr` 渐变文件获取颜色。
- 线性、圆锥和方形渐变的 **角度**，[[docker.edge_pad]]（每一端有多少为纯色），以及射线、圆锥和方形渐变的中心偏移。
- [[docker.repeat]]：[[docker.repeat_none]]、[[docker.repeat_sawtooth]]（颜色从头再来一遍）或 [[docker.repeat_mirror]]（颜色来回往复），并设置 [[docker.repeat_count]]。
- [[docker.bilinear]]：颜色从第一种到最后一种，再返回。
- [[docker.invert_colours]]：颜色按相反顺序排列。

类型下方的色条按绘制效果显示整个颜色序列。

## 编辑填充对话框 {#edit-fill}

<kbd>F11</kbd> 打开所选对象填充的 [[edit_fill.title]] 对话框。渐变会显示预览、[[edit_fill.fill_picker]]、类型、重复方式和色带：单击节点选中，拖动移动，双击添加或删除节点。色带下方是节点的颜色、透明度、位置、[[edit_fill.midpoint]] 和 [[edit_fill.reverse]]。[[edit_fill.transform]] 包含旋转、中心和边缘填充。**确定** 应用您所做的更改，并包括 [[edit_fill.overprint]]。对话框打开期间，所选对象会随着你的修改实时显示填充；**取消** 或 <kbd>Esc</kbd> 会恢复原来的填充。

![编辑填充对话框](../../img/color-fills-edit-fill.png)

## 交互式填充工具 {#interactive-fill}

[[tool.interactive_fill]] 工具（<kbd>G</kbd>）直接在对象上编辑填充。

1. 选择该工具并在对象上拖动：从按下处到释放处得到一个线性渐变。按住 <kbd>Ctrl</kbd> 可将角度限制为 15 度的倍数。单击则得到默认填充。
2. 拖动对象上的节点以更改渐变：线性渐变的两个方形节点改变其方向和长度；射线、圆锥或方形渐变的节点移动其中心。
3. 将调色板颜色拖到节点上可为其重新着色，拖到节点之间的线上可在该处添加一种颜色。

属性栏上有填充类型及其设置：均匀填充的颜色；渐变的类型、起始和结束颜色、角度和边缘填充。

## 区域填充工具 {#area-fill}

[[tool.area_fill]] 工具通过为任何闭合区域创建一个新对象来填充它，即使该区域由多个重叠对象构成。周围的对象保持原样。

1. 选择 [[tool.area_fill]] 工具。
2. 在属性栏上选择填充和轮廓：**指定**（旁边的颜色，轮廓还有宽度）、**使用默认值**（新对象的填充和轮廓；若默认为无填充，则使用一种颜色，以便区域可见）或 **无**。
3. 在区域内部单击。

新对象放在活动图层的顶部并被选中。在轮廓上单击，或在没有被任何对象围住的区域中单击，不会创建任何对象。

## 网状填充 {#mesh-fill}

网状填充是覆盖在对象上的节点网格，每个节点都有自己的颜色和透明度；颜色在节点之间平滑流动。它适合表现逼真的明暗。

1. 选择 [[tool.mesh_fill]] 工具（<kbd>M</kbd>），在属性栏上设置行数和列数（起初为 2 x 2，最多 50）。
2. 单击一个对象。它会得到一个与其填充颜色相同的网格。
3. 单击节点将其选中，然后单击调色板中的颜色为其着色。拖动节点可弯曲颜色的流向。
4. 在网格内部双击可添加一条经过最近节点的行或列；选择一个节点并按 <kbd>Delete</kbd> 可删除其所在的行和列。

属性栏上还有 **平滑颜色**（使颜色混合得更柔和）、所选节点的 **透明度**，以及删除网状填充的 **清除网状**。更改网格的行数和列数会保留其颜色。

## 图样和底纹填充 {#pattern-texture}

- [[docker.fill_pattern]] 填充重复一个图块：双色、矢量图样或位图。请参阅 [图样填充](patterns.md)。
- [[docker.fill_texture]] 填充是生成的自然外观纹理：[[texture.clouds]]、[[texture.marble]]、[[texture.noise]] 或 [[texture.wood]]，可设置其颜色和比例。

## 一个对象上的多个填充 {#several-fills}

在 [[docker.properties]] 面板的填充部分，[[docker.add_fill]] 在对象自身的填充之上再绘制一个填充，起初为可修改的副本。后续填充显示为“填充 2”“填充 3”等，每个都有自己的编辑器和一个删除按钮。轮廓部分也可以用同样的方式添加更多轮廓。

填充也可以绘制在轮廓上；请参阅 [设置轮廓](../lines-shapes/outlines.md#outline-pen)。
