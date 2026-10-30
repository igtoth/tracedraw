# 填色

填色會為封閉物件的內部上色。TraceDraw 有六種填色：[[docker.fill_none]]、[[docker.fill_uniform]]（單一平面色彩）、[[docker.fill_fountain]]（彼此融合的色彩）、[[docker.fill_pattern]]、[[docker.fill_texture]] 和 [[docker.fill_mesh]]（在格網節點之間流動的色彩）。

## 填滿物件的方式 {#ways}

- 按一下調色盤中的色彩，為選取範圍套用均勻填色。
- 使用 [[docker.properties]] 面板（<kbd>F11</kbd>）的 [[docker.fill]] 區段：以那一排圖片選擇填色種類，然後設定其選項。
- 使用 [[tool.interactive_fill]] 工具（<kbd>G</kbd>）直接在物件上設定漸層。
- 使用 [[tool.area_fill]] 工具填滿由多個物件圍成的區域。
- 使用 [[tool.mesh_fill]] 工具（<kbd>M</kbd>）建立網狀填色。

沒有選取任何物件時，您選擇的填色會成為新物件的填色。

## 均勻填色 {#uniform}

均勻填色是單一色彩，可使用任何色彩模型。從調色盤、[[docker.color]] 面板（請參閱[色彩面板](color-docker.md)）或 [[docker.properties]] 面板的填色區段中選擇。

## 漸層填色 {#gradient}

漸層會融合兩種以上的色彩。其設定位於 [[docker.properties]] 面板的填色區段中：

- 類型：[[fill.linear]]（沿著直線）、[[fill.radial]]（從中心向外）、[[fill.conical]]（繞著中心）、[[fill.square]]（以逐漸擴大的方形）或 [[fill.shaped]]（依循物件的外框，從邊緣向內）。
- [[docker.gradient_presets]]：可作為起點的現成漸層。
- **色標**：色彩及其在漸層上的位置，數量為 2 到 64 個。選取一個色標可變更其色彩或位置；[[docker.add_stop]] 和 [[docker.remove_stop]] 可新增和移除色標。[[docker.import_gradient]] 會從 `.ggr` 漸層檔案取得色彩。
- 線性、圓錐和方形漸層的**角度**、[[docker.edge_pad]]（兩端各有多少為實色），以及放射狀、圓錐和方形漸層的中心偏移。
- [[docker.repeat]]：[[docker.repeat_none]]、[[docker.repeat_sawtooth]]（色彩從頭再跑一次）或 [[docker.repeat_mirror]]（色彩來回往返），並可設定 [[docker.repeat_count]]。
- [[docker.bilinear]]：色彩從第一個跑到最後一個再返回。
- [[docker.invert_colours]]：色彩以相反順序排列。

類型下方的色條會顯示將要繪製的完整色彩排列。

## 編輯填色對話方塊 {#edit-fill}

<kbd>F11</kbd> 開啟所選物件填色的 [[edit_fill.title]] 對話方塊。漸層會顯示預覽、[[edit_fill.fill_picker]]、類型、重複方式和色帶：按一下節點選取，拖曳移動，按兩下新增或刪除節點。色帶下方是節點的顏色、透明度、位置、[[edit_fill.midpoint]] 和 [[edit_fill.reverse]]。[[edit_fill.transform]] 包含旋轉、中心和邊緣填補。**確定** 套用您所做的變更，並包括 [[edit_fill.overprint]]。對話方塊開啟期間，所選物件會隨著你的修改即時顯示填色；**取消** 或 <kbd>Esc</kbd> 會還原原本的填色。

![編輯填色對話方塊](../../img/color-fills-edit-fill.png)

## 互動式填色工具 {#interactive-fill}

[[tool.interactive_fill]] 工具（<kbd>G</kbd>）可直接在物件上編輯填色。

1. 選擇此工具並在物件上拖曳：您會得到一個從按下處到放開處的線性漸層。按住 <kbd>Ctrl</kbd> 可讓角度以 15 度為增量。按一下則會套用預設填色。
2. 拖曳物件上的節點以變更漸層：線性漸層的兩個方形節點可轉動其方向和長度；放射狀、圓錐或方形漸層的節點可移動其中心。
3. 將調色盤色彩拖曳到節點上可重新上色，拖曳到節點之間的線上則可在該處新增色彩。

屬性列包含填色種類及其設定：均勻填色的色彩；漸層則有類型、起始和結束色彩、角度以及邊緣填補。

## 區域填色工具 {#area-fill}

[[tool.area_fill]] 工具可填滿任何封閉區域，即使是由多個重疊物件形成的區域也可以，方法是以該區域建立一個新物件。周圍的物件保持不變。

1. 選擇 [[tool.area_fill]] 工具。
2. 在屬性列上選擇填色和外框：**指定**（旁邊的色彩，外框則還有寬度）、**使用預設值**（新物件的填色和外框；若預設為無填色，則使用一種色彩，讓區域能夠顯示）或**無**。
3. 在區域內按一下。

新物件會放在作用中圖層的最上層並被選取。在外框上按一下，或在沒有任何物件圍住的區域中按一下，都不會產生任何物件。

## 網狀填色 {#mesh-fill}

網狀填色是覆蓋在物件上的節點格網，每個節點都有自己的色彩和透明度；色彩會在節點之間平滑流動。它適合用於寫實的陰影。

1. 選擇 [[tool.mesh_fill]] 工具（<kbd>M</kbd>），並在屬性列上設定列數和欄數（預設為 2 x 2，最多 50）。
2. 按一下物件。物件會得到一個與其填色同色的網格。
3. 按一下節點以選取它，然後按一下調色盤中的色彩為其上色。拖曳節點可彎曲色彩的流動。
4. 在網格內按兩下，可在最近的節點處新增一列或一欄；選取節點並按 <kbd>Delete</kbd> 可移除其所在的列和欄。

屬性列還有**平滑色彩**（讓色彩融合得更柔和）、所選節點的**透明度**，以及**清除網格**（移除網狀填色）。變更網格的列數和欄數時會保留其色彩。

## 圖樣和材質填色 {#pattern-texture}

- [[docker.fill_pattern]] 填色會重複拼貼：雙色、向量圖樣或點陣圖。請參閱[圖樣填色](patterns.md)。
- [[docker.fill_texture]] 填色是自動產生、外觀自然的材質：[[texture.clouds]]、[[texture.marble]]、[[texture.noise]] 或 [[texture.wood]]，並可設定其色彩和比例。

## 一個物件上的多重填色 {#several-fills}

在 [[docker.properties]] 面板的填色區段中，[[docker.add_fill]] 會在物件本身的填色上再繪製另一個填色，一開始是可供修改的複本。其他填色會顯示為「填色 2」、「填色 3」等，每個都有自己的編輯器和移除按鈕。外框區段也可以用同樣的方式新增更多外框。

填色也可以繪製在外框上；請參閱[設定外框](../lines-shapes/outlines.md#outline-pen)。
