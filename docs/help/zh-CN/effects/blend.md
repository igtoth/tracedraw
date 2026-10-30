# 调和

调和在两个对象之间绘制一系列中间步骤，形状、大小和颜色从一个对象逐渐变为另一个。可以用它制作平滑的明暗、重复的图案，或从一个形状变为另一个形状。

## 调和两个对象 {#create}

使用工具：

1. 从效果展开工具栏中选择 [[tool.blend]] 工具。
2. 从起始对象拖动到结束对象。

使用面板：

1. 选择两个对象。
2. 使用 [[menu.effects > menu.effects.blend]]（<kbd>Ctrl+F8</kbd>）打开 [[docker.blend]] 面板。
3. 设置选项并单击 [[docker.apply]]。

![调和面板](../../img/effects-blend.png)

两个对象仍然是绘图中的对象：移动、调整大小或重新着色其中任何一个，步骤都会随之改变。删除结束对象会删除调和。

## 调和设置 {#settings}

选择调和时，属性栏或面板中的每项更改都会立即应用。

- [[docker.blend_by_steps]]：步数（起初为 5，最多 999）。
- **间距**，用于沿路径的调和：步骤之间的距离；步数由路径长度决定。
- **旋转**：逐步旋转各步骤；**循环** 则改为绕两个对象之间的中点旋转。
- **加速**：对象滑块使步骤向一端聚集；颜色滑块对颜色执行相同操作。[[docker.blend_apply_sizing]] 使对象加速同时改变大小，[[docker.blend_link_accel]] 使两个滑块保持一致。**重置** 将两者恢复为均匀。
- [[docker.blend_color_blend]]：[[docker.blend_direct]]（从一种颜色直接到另一种）、[[docker.blend_clockwise]] 或 [[docker.blend_counterclockwise]]（沿色轮绕行）。

工具的属性栏上有步数、旋转和两种加速。

## 沿路径调和 {#path}

1. 选择调和。
2. 在 [[docker.blend]] 面板中单击 **新路径**，然后单击一条曲线。

步骤沿曲线排列。**旋转全部对象** 使每个步骤沿曲线转向，[[docker.blend_full_path]] 将第一个和最后一个步骤放在曲线的两端。

## 删除或拆分调和 {#clear}

**清除调和** 删除各步骤并保留两个对象。**拆分** 将各步骤变为独立的对象。
这些步骤会编组放在对象旁边。[[menu.object > menu.object.break_blend_apart]]（<kbd>Ctrl+K</kbd>，右键菜单中也有）效果相同。
