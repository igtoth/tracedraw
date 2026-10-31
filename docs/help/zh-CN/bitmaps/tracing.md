# 描摹位图

描摹将位图变为矢量形状，你可以编辑、重新着色和缩放它们而不损失质量：扫描的徽标、草图、照片都可以。

## 快速描摹 {#quick-trace}

选择一个位图并选择 [[menu.bitmaps > menu.bitmaps.quick_trace]]。它使用全局选项中 [[options.bitmap_tracing]] 页面上选定的预设（起初为 [[menu.bitmaps.trace_optimized]]，它为每张图片选定细节：在节点最少的情况下尽可能忠实于图片）。

[[menu.bitmaps.repeat_last_trace]] 不打开对话框，按上次的方式再次描摹所选位图；[[menu.bitmaps.last_trace_settings]] 以上次的设置打开对话框。位图的右键菜单中也有这些命令，以及 [[menu.bitmaps.centerline_trace]] 和 [[menu.bitmaps.outline_trace]]。

## 描摹位图对话框 {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] 和 [[menu.bitmaps.centerline_trace]] 列出预设；选择一个即可打开描摹对话框。

- **轮廓描摹** 用形状填充颜色区域：[[menu.bitmaps.trace_line_art]]、[[menu.bitmaps.trace_logo]]、[[menu.bitmaps.trace_detailed_logo]]、[[menu.bitmaps.trace_clipart]]、[[menu.bitmaps.trace_low_quality]]、[[menu.bitmaps.trace_high_quality]] 和 [[menu.bitmaps.trace_optimized]]。
- **中心线描摹** 用开放曲线沿线条描摹：[[menu.bitmaps.trace_technical]] 和 [[menu.bitmaps.trace_line_drawing]]。

![描摹位图对话框](../../img/bitmaps-tracing.png)

对话框是一个可以调整大小和最大化的窗口。预览占据左侧，设置位于右侧的面板中，每当你更改设置时都会重新描摹：无需按任何按钮即可看到结果。预览下方的一行统计描摹结果的 **曲线**、**节点** 和 **颜色** 数。单击 [[dialog.ok]] 将形状放在位图上方（最暗的在最先），或单击 [[dialog.cancel]] 保持位图不变；这两个按钮位于对话框的右下角。

## 预览 {#preview}

[[dialog.trace_preview_label]] 列表选择预览显示的内容：

- [[dialog.trace_view_before_after]]：位图在上，描摹结果在下；或 [[dialog.view_side_by_side]]；[[dialog.before]] 只显示位图。
- [[dialog.trace_view_large]]：只显示描摹结果，和预览一样大。
- [[dialog.trace_view_wireframe]]：描摹结果的曲线以细线画在位图上。列表旁边出现 [[dialog.trace_overlay]] 滑块：它设置位图在线条下显示的强度，从 0（只有线条）到 100（位图原样），起初为 80。

列表后面的按钮分别是缩放到适合、缩小、放大和平移。滚轮围绕指针缩放，拖动则移动图片；两个窗格一起移动。透明部分显示棋盘格。

在浏览器中，大图片的预览由一个较小的副本生成，预览下方会有一行红色的说明提示这一点：那里的细微细节可能显得粗糙，但 [[dialog.ok]] 会描摹整张图片。桌面版的预览则完全准确。

## 设置 {#settings}

[[dialog.trace_tab_settings]] 选项卡包含：

- [[dialog.trace_type]] 轮廓或中心线，以及 [[dialog.trace_image_type]] 预设。选择一个预设会把下面的滑块设为适合该类图片的值。
- [[dialog.detail]]（0 到 100）：细节越多，保留的小区域和细线越多；细节越少，斑点和颗粒越平滑。使用 [[menu.bitmaps.trace_optimized]] 时，细节会为每张图片选定，滑块显示为灰色。
- [[dialog.smoothing]]（0 到 100，起初为 25）：曲线更平滑，节点更少。
- [[dialog.corner_smoothness]]（0 到 100）：拐角更圆。
- [[dialog.trace_remove]]：从描摹结果中省去什么。**背景颜色** 删除到达图片边缘的背景，**图像中的整种颜色** 删除该颜色的每个区域，**无** 描摹所有内容。[[dialog.trace_remove_auto]] 沿图片边缘查找背景颜色；[[dialog.trace_remove_specify]] 使用色样的颜色，你可以更改它，或用旁边的吸管从预览中拾取。
- [[dialog.trace_objects]]：[[dialog.trace_merge_adjacent]]（起初打开）把一种颜色的所有区域合成一条曲线；关闭时，每个独立的色块是一条单独的曲线。[[dialog.trace_remove_overlap]] 让每个形状只占自己的像素，保留孔洞；关闭时，形状完整地画在其内部形状之下（头部在眼睛之下）。无论哪种情况，相邻形状沿边界共用一条曲线，因此不会出现缝隙，线框中每条边界只显示一次。[[dialog.trace_group_by_color]] 把每种颜色的曲线放进各自的群组；当一种颜色的区域尚未合并时可用。
- [[dialog.delete_original]]：放置描摹结果时删除位图。

不适用于当前描摹类型的设置显示为灰色：中心线描摹只读取细节和平滑。

对话框左下角的 [[dialog.trace_optimize]] 在图片上尝试多个细节，将每次的描摹结果与图片比较，然后把以最少节点最忠实地绘制图片的细节放入 [[dialog.detail]] 滑块。这是一个可以撤消的步骤。如果在它完成之前更改了设置，它的结果会被舍弃。

## 颜色 {#colors}

[[dialog.trace_tab_colors]] 选项卡设置 [[dialog.color_mode]]（RGB、灰度或黑白）和 [[dialog.trace_number_of_colors]]（2 到 256），并列出描摹结果的颜色及每种颜色的曲线数。勾选两种或更多颜色并单击 [[dialog.trace_merge_colors]]，可把它们合并到第一种颜色；单击某种颜色的色样，可用另一种颜色绘制它的曲线。更改会重新描摹图片的设置时，这些属于上一次描摹的编辑会被忘记。

## 重置、撤消和重做 {#steps}

在设置面板下方，[[dialog.reset]] 恢复预设的设置，两个箭头在对话框中所做的每项更改之间前后移动：每次松开滑块、选择列表项、勾选复选框、编辑颜色和重置都是一步。撤消后所做的更改会丢弃已撤消的步骤。

[[dialog.trace_options]] 打开全局选项的 [[options.bitmap_tracing]] 页面，你可以在那里选择快速描摹的预设以及每次描摹的起始选项，然后返回对话框。[[dialog.help_button]] 按钮打开此页面。
