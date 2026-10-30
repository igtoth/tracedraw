# 位图效果

位图效果改变像素：模糊、锐化、调整颜色、添加纹理和艺术笔触。它们作用于位图，也作用于矢量对象；矢量对象随后被绘制为效果生成的图片，同时保留自身的形状、填充和轮廓。

效果与原始像素分开保存：你以后可以编辑、隐藏、重新排序或删除其中任何一个，删除最后一个效果即可恢复原图。

## 应用位图效果 {#apply}

1. 选择一个或多个对象。
2. 在 [[menu.effects]] 菜单中选择一个组和一种效果。
3. 没有设置的效果会立即应用。其他效果会打开一个对话框，显示第一个所选对象的 **之前** 和 **之后** 预览：设置数值后单击 [[dialog.ok]]。设置位于右侧的面板中；面板下方的 **重置** 将所有设置恢复为默认值，两个箭头撤消和重做在对话框中所做的更改。

![位图效果对话框](../../img/effects-bitmap-effects.png)

对整个所选内容的更改为一个撤消步骤。选择位图时，<kbd>Ctrl+B</kbd> 打开 [[fx.brightness_contrast_intensity]]，<kbd>Ctrl+Shift+B</kbd> 打开 [[fx.color_balance]]，<kbd>Ctrl+Shift+U</kbd> 打开 [[fx.hue_saturation_lightness]]。

## 效果组 {#groups}

| 组 | 效果 |
|---|---|
| [[menu.effects.3d]] | [[fx.cylinder]]、[[fx.emboss]]、[[fx.page_curl]]、[[fx.pinch_punch]]、[[fx.sphere]] |
| [[menu.effects.adjust]] | [[fx.auto_adjust]]、[[fx.image_adjustments]]、[[fx.contrast_enhancement]]、[[fx.local_equalization]]、[[fx.target_balance]]、[[fx.tone_curve]]、[[fx.brightness_contrast_intensity]]、[[fx.color_balance]]、[[fx.gamma]]、[[fx.hue_saturation_lightness]]、[[fx.selective_color]]、[[fx.replace_colors]]、[[fx.desaturate]]、[[fx.channel_mixer]] |
| [[menu.effects.art_strokes]] | [[fx.charcoal]]、[[fx.conte_crayon]]、[[fx.crayon]]、[[fx.cubist]]、[[fx.impressionist]]、[[fx.palette_knife]]、[[fx.pastels]]、[[fx.pen_ink]]、[[fx.pointillist]]、[[fx.scraperboard]]、[[fx.sketch_pad]]、[[fx.watercolor]]、[[fx.felt_marker]]、[[fx.paper_grain]] |
| [[menu.effects.blur]] | [[fx.directional_smooth]]、[[fx.gaussian_blur]]、[[fx.jagged_edges]]、[[fx.low_pass]]、[[fx.motion_blur]]、[[fx.radial_blur]]、[[fx.smart_blur]]、[[fx.smooth]]、[[fx.soften]]、[[fx.zoom]] |
| [[menu.effects.camera]] | [[fx.colorize]]、[[fx.diffuse]]、[[fx.photo_filter]]、[[fx.sepia_toning]]、[[fx.vintage_photo]] |
| [[menu.effects.color_transform]] | [[fx.bit_planes]]、[[fx.halftone]]、[[fx.psychedelic]]、[[fx.solarize]] |
| [[menu.effects.contour_group]] | [[fx.edge_detect]]、[[fx.find_edges]]、[[fx.trace_contour]] |
| [[menu.effects.correction]] | [[fx.dust_and_scratch]] |
| [[menu.effects.creative]] | [[fx.crystallize]]、[[fx.fabric]]、[[fx.frame]]、[[fx.glass_tiles]]、[[fx.mosaic]]、[[fx.scatter]]、[[fx.tinted_glass]]、[[fx.stained_glass]]、[[fx.vignette]]、[[fx.vortex]] |
| [[menu.effects.custom]] | [[fx.bump_map]] |
| [[menu.effects.distort]] | [[fx.blocks]]、[[fx.displace]]、[[fx.mesh_warp]]、[[fx.offset]]、[[fx.pixelate]]、[[fx.ripple]]、[[fx.swirl]]、[[fx.tile]]、[[fx.wet_paint]]、[[fx.whirlpool]]、[[fx.wind]] |
| [[menu.effects.noise]] | [[fx.add_noise]]、[[fx.maximum]]、[[fx.median]]、[[fx.minimum]]、[[fx.remove_moire]]、[[fx.remove_noise]] |
| [[menu.effects.sharpen]] | [[fx.adaptive_unsharp]]、[[fx.directional_sharpen]]、[[fx.high_pass]]、[[fx.sharpen]]、[[fx.unsharp_mask]] |
| [[menu.effects.texture]] | [[fx.cobblestone]]、[[fx.wrinkles]]、[[fx.etching]]、[[fx.plastic]]、[[fx.relief_sculpture]]、[[fx.stone]] |
| [[menu.effects.transform]] | [[fx.deinterlace]]、[[fx.invert_colors]]、[[fx.posterize]] |

## 特殊对话框 {#special-dialogs}

- **[[fx.image_adjustments]]** 打开一个较大的对话框：一个可缩放、平移和旋转的大预览，可单独显示、并排显示或拆分显示；**自动调整** 以及白点和黑点拾取器；色温、色调、饱和度、亮度、对比度、高光、阴影和中间调滑块，并带有直方图；对话框内的撤消和重做；以及可供比较的设置快照。
- **[[fx.contrast_enhancement]]** 显示直方图，并带有可拖动的输入和输出色阶标记、黑色和白色滴管以及 **自动调整**。
- **[[fx.tone_curve]]** 在直方图上按通道（主通道、红、绿、蓝）编辑曲线，有四种样式（曲线、直线、手绘、伽玛）；曲线可以保存和加载。
- 图片的颜色（例如 [[fx.replace_colors]] 中的颜色）带有滴管：单击预览即可获取指针下的颜色。

## 编辑对象的效果 {#fx-section}

[[docker.properties]] 面板的 [[docker.fx]] 部分列出所选对象的效果，最先应用的在顶部。每一行都有一个用于显示或隐藏效果的复选框、**编辑**（打开其对话框）、**上移**、**下移** 和 **删除**。**添加效果** 打开效果组。

[[menu.effects > menu.effects.flatten]] 使效果成为永久性的：位图保留其显示的内容，矢量对象变为位图。

## 效果选择器 {#effect-picker}

[[menu.effects > menu.effects.picker]] 打开一个在你工作时保持打开的窗口，每种效果都显示为一张小图片。在搜索字段中输入可查找效果，或从列表中选择一个组或 **最近**。选中某些内容时，单击图块即可应用该效果。你也可以将效果作为 [透镜](lens.md) 应用。

## 扩展到边缘以外的效果 {#inflate}

模糊和类似效果可能延伸到位图边缘以外。当 [[menu.bitmaps > menu.bitmaps.inflate > menu.bitmaps.inflate_auto]] 打开时（起初即打开），位图会扩大以容纳它们。请参阅 [重新取样和矫正](../bitmaps/resample-and-straighten.md#inflate)。

要将带有效果的位图保存为 PNG 文件，请右键单击它并选择 [[menu.bitmaps.save_bitmap_as]]。
