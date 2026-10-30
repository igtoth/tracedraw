# 透明度

透明度讓物件下方的內容能透過物件顯示出來。它可以處處相同，也可以像漸層一樣在物件上變化，或依循圖樣或材質。

## 套用透明度 {#apply}

- 使用 [[tool.transparency]] 工具：按一下物件，然後在屬性列上選擇透明度種類，或在物件上拖曳以建立從不透明（按下處）到透明（放開處）的漸層透明度。之後漸層的節點會像 [[tool.interactive_fill]] 工具那樣顯示在物件上：拖曳起點、終點或圓形傾斜節點；拖曳兩個節點之間的橫條（中點）可改變過渡到一半的位置；按兩下線條新增節點，按兩下節點將其移除。
- 在 [[docker.properties]] 面板的 [[docker.transparency]] 區段中設定。

## 設定 {#settings}

- 種類：[[transparency.none]]、[[transparency.uniform]]（預設）、[[transparency.fountain]]（線性、放射狀、圓錐、方形或造形，與漸層填色相同）、[[transparency.pattern]] 或 [[transparency.texture]]。
- 程度：0 到 100%（預設為 50%）。對於漸層，色標是灰色色階：白色為不透明，黑色為完全透明。
- [[transparency.merge_mode]]：色彩與下方內容的混合方式。一般（預設）只是單純透出；其他模式，例如相加、相減、差異化、色彩增值、相除、若較亮、若較暗、色相、飽和度、色彩和反轉，則以其他方式組合色彩。
- 套用至 [[transparency.target_fill]]、[[transparency.target_outline]] 或 [[transparency.target_all]]。
- **凍結**：保留目前透出的內容，即使下方的物件移動也不改變。

要移除透明度，請按一下屬性列末端的 [[toolbar.clear_transparency]]（或將類型選為 [[transparency.none]]）。

若要複製另一個物件的透明度，請使用 [[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]]。關於由物件亮度定義的透明區域，請參閱[不透明度遮罩](../objects/clip-frames.md#opacity-mask)。
