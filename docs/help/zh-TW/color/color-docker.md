# 色彩面板

[[docker.color]] 面板（<kbd>Shift+F11</kbd>）可調配色彩，並將其套用到選取範圍的填色或外框。[[tool.outline_color]] 工具（<kbd>Shift+F12</kbd>）也會開啟它。從 [[menu.window > menu.window.dockers]] 開啟它。

![色彩面板](../../img/color-color-docker.png)

## 選擇色彩 {#choose}

1. 選取物件。面板會從第一個物件的填滿色彩開始。
2. 在其中一個檢視中調配新色彩：
   - [[docker.color_viewers]]：在方塊中按一下或拖曳以設定飽和度（由左至右）和亮度（由下至上），在其旁邊的色條中設定色相；
   - [[docker.color_sliders]]：[[docker.model]] 的每個組成分量各有一個滑桿：RGB、CMYK（預設）、HSB、HSL、Lab、灰階或 YIQ；
   - [[docker.color_palettes]]：調色盤的色樣。

   頂端的三個按鈕可顯示或隱藏各個檢視；可以任意組合顯示。您也可以輸入 `#rrggbb` 值。
3. 按一下面板底部的 [[docker.fill]] 或 [[docker.outline]] 以套用新色彩。鎖（[[docker.lock_colors]]）把填色和外框綁在一起，任一按鈕都會把色彩同時套用到兩者。"..." 按鈕（[[docker.more_actions]]）展開其他動作：[[docker.no_fill]], [[docker.no_outline_action]], [[docker.swap_colors]], [[docker.add_to_document_palette]], [[docker.add_to_spot_colors]] 和 [[docker.add_to_color_styles]]。

舊色彩和新色彩會並排顯示。當新色彩無法以 CMYK 油墨精確印出時，面板會顯示警告。

## 取樣色彩 {#sample}

按一下 [[docker.eyedropper]]，然後按一下繪圖上的任何位置以取得該處的色彩。接著按一下 [[docker.use_sampled]]，使其成為新色彩。另請參閱[滴管](eyedroppers.md)。

## 特別色油墨 {#spot-inks}

在按鈕下方，[[docker.spot_inks]] 會列出繪圖的特別色油墨；請參閱[特別色](spot-colors.md)。
