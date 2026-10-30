# 透明度

透明度让对象下方的内容透过它显示出来。它可以处处相同，也可以像渐变一样在对象上变化，或者遵循图样或底纹。

## 应用透明度 {#apply}

- 使用 [[tool.transparency]] 工具：单击对象，然后在属性栏上选择透明度类型；或在对象上拖动，得到从不透明（按下处）到透明（释放处）的渐变透明度。随后渐变的节点会像 [[tool.interactive_fill]] 工具那样显示在对象上：拖动起点、终点或圆形倾斜节点；拖动两个节点之间的横条（中点）可改变过渡到一半的位置；双击线条添加节点，双击节点将其删除。
- 在 [[docker.properties]] 面板的 [[docker.transparency]] 部分中设置。

## 设置 {#settings}

- 类型：[[transparency.none]]、[[transparency.uniform]]（起初选用）、[[transparency.fountain]]（线性、射线、圆锥、方形或形状，与渐变填充相同）、[[transparency.pattern]] 或 [[transparency.texture]]。
- 数量：0 到 100%（起初为 50%）。对于渐变，色标为灰色阴影：白色为不透明，黑色为完全透明。
- [[transparency.merge_mode]]：颜色如何与下方内容混合。常规（起初选用）只是透出下方内容；其他模式（例如添加、减少、差异、乘、除、如果更亮、如果更暗、色度、饱和度、颜色和反转）以其他方式组合颜色。
- 应用于 [[transparency.target_fill]]、[[transparency.target_outline]] 还是 [[transparency.target_all]]。
- **冻结**：将透出的内容固定为当前状态，即使下方对象移动也不变。

要去掉透明度，请点击属性栏末尾的 [[toolbar.clear_transparency]]（或将类型选为 [[transparency.none]]）。

要复制另一个对象的透明度，请使用 [[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]]。关于由对象亮度定义的透明区域，请参阅 [不透明度蒙版](../objects/clip-frames.md#opacity-mask)。
