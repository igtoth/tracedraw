# 更改位图的颜色模式

> *机器翻译：本页可能落后于英文版。*

[[menu.bitmaps > menu.bitmaps.mode]] 将所选位图转换为另一种颜色模式。所有所选位图的更改为一个撤消步骤，并保留透明度。带有设置的模式会打开一个对话框，显示转换前后的预览；设置位于右侧的面板中，面板下方的 [[dialog.reset]] 恢复设置，两个箭头撤消和重做在对话框中所做的更改。

| 模式 | 结果 |
|---|---|
| [[menu.bitmaps.mode_bw]] | 只有黑色和白色（见下文） |
| [[menu.bitmaps.mode_grayscale]] | 灰色阴影 |
| [[menu.bitmaps.mode_duotone]] | 一到四种油墨（见下文） |
| [[menu.bitmaps.mode_paletted]] | 最多 256 种颜色（见下文） |
| [[menu.bitmaps.mode_rgb]] | 位图的模式变为 RGB；像素保持原样 |
| [[menu.bitmaps.mode_lab]] | Lab 颜色 |
| [[menu.bitmaps.mode_cmyk]] | 位图的模式变为 CMYK，其像素显示油墨能够印出的颜色，由颜色管理配置文件计算 |

所有选中的位图都处于某一模式时，菜单会勾选 [[menu.bitmaps.mode_rgb]] 或 [[menu.bitmaps.mode_cmyk]]，状态栏会显示所选位图的模式，例如“Bitmap 600 x 400 px (CMYK)”。`.cdr` 文件中的 CMYK 图片以 CMYK 模式打开，`.tdraw` 文件会保留该模式。[[menu.tools > menu.tools.convert_colors]] 也会以同样方式设置所转换的每个位图的模式（参见[转换颜色](../color/convert-colors.md)）。

## 黑白 {#black-and-white}

![黑白对话框](../../img/bitmaps-color-modes-black-and-white.png)

选择转换方法：

- **线条图**：灰度等于或高于 **阈值**（起初为 128）处为白色，其他地方为黑色；
- **顺序**：规则的点图案；
- **半色调**（起初选用）：以一定 **角度**（45 度）和 **每英寸线数**（60）排列的圆点、线条或方点网屏；
- **基数分布**：看似随机的纹理；
- **Jarvis**、**Stucki** 和 **Floyd-Steinberg**：误差扩散，对照片效果最平滑。

**强度**（0 到 100，起初为 50）使结果更亮或更暗。

## 双色调 {#duotone}

![双色调对话框](../../img/bitmaps-color-modes-duotone.png)

双色调用少数几种油墨印刷灰度图片，以很少的颜色得到丰富的色调。

1. 选择类型：单色调、双色调（起初选用）、三色调或四色调，对应一到四种油墨。
2. 单击一种油墨以更改其颜色。
3. 编辑其曲线：横向是灰度级别（左侧为黑色），纵向是该油墨的印刷量。在空白处拖动可添加点，拖动点可移动它，双击点或将其拖出网格可删除它。**全部显示** 同时绘制其他油墨的曲线。
4. **保存** 和 **加载** 将油墨和曲线保存在文件中。

## 调色板 {#paletted}

![调色板对话框](../../img/bitmaps-color-modes-paletted.png)

- **调色板**：均匀颜色、标准 VGA（16 色）、自适应和优化（图片自身的颜色，减少到你设置的 **颜色** 数，最多 256）、灰度、系统或自定义（文档调色板）。
- **抖动**：无、顺序、Jarvis、Stucki 或 Floyd-Steinberg，并可设置抖动强度。

生成的调色板显示在设置下方。

## 为黑白位图着色 {#monochrome}

只选择双色位图时，单击调色板颜色可为其背景着色，右键单击可为其前景着色。
