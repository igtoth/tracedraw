# 調色盤

沿著視窗底部排列的調色盤是為物件上色最快的方法。在它下方，**文件調色盤**會收集您的繪圖所使用的色彩。您可以開啟更多調色盤、建立自己的調色盤並加以編輯。

## 使用調色盤 {#using}

| 動作 | 結果 |
|---|---|
| 按一下色彩 | 填滿選取範圍（沒有選取任何物件時，設定新物件的填色） |
| 以右鍵按一下色彩 | 為選取範圍的外框上色 |
| <kbd>Ctrl</kbd>+按一下色彩 | 將該色彩的十分之一混入選取範圍的填色 |
| 按住色彩 | 顯示該色彩較淺、較深、較暗淡和較鮮豔的色調供選擇 |
| 將色彩拖曳到物件上 | 填滿物件，或放在邊緣上時為其外框上色 |
| 按一下有叉號的色塊 | 移除填色（以右鍵按一下：移除外框） |

兩端的箭頭可捲動調色盤，右端的按鈕則可一次顯示所有色彩。將指標停留在色彩上，即可查看其名稱和數值。左端的調色盤選單按鈕會開啟調色盤選單。

調色盤可以移到視窗的其他邊緣，或設為浮動：拖曳其虛線握把即可。

在每一側，它可以停駐在視窗邊緣，也可以緊鄰繪圖區，位於繪圖區與該側的面板或工具箱之間。拖曳時，藍色條帶會顯示它將停駐的位置；在其握把上按右鍵可從清單中選擇位置。

## 文件調色盤 {#document-palette}

您套用到物件的每個色彩都會加入文件調色盤，它會隨繪圖一起儲存。其滴管會加入指標下方的色彩，其選單包含**自動更新**（可停止自動加入色彩）、**從選取範圍新增**、**從文件新增**、**刪除色彩**和**重設調色盤**（只保留繪圖仍在使用的色彩）。您也可以將色彩或物件拖曳到它上面。若要顯示或隱藏它，請參閱[自訂選項](../customize/customization.md#color-palette)。

## 調色盤面板 {#palettes-docker}

[[docker.palettes]] 面板會列出您擁有的調色盤，以及每個調色盤中的色彩數量。使用 [[menu.window > menu.window.color_palettes > menu.window.palette_manager]] 開啟它。

![調色盤面板](../../img/color-palettes.png)

- 勾選調色盤即可將其顯示在繪圖視窗下方。
- [[docker.edit]] 會在調色盤編輯器中開啟它。
- [[menu.window.palette_open]] 會開啟調色盤檔案（`.tdpal`、`.gpl`、`.ase` 或 `.aco`）。
- [[menu.window.palette_from_document]] 和 [[menu.window.palette_from_selection]] 會以繪圖或選取範圍中使用的色彩建立調色盤。
- 文件調色盤的色彩顯示在底部；按一下可填滿選取範圍，以右鍵按一下可為外框上色。

## 編輯調色盤 {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]] 會開啟調色盤編輯器。

![調色盤編輯器](../../img/color-palettes-palette-editor.png)

1. 選擇要編輯的 [[dialog.palette]]，或按一下 [[dialog.new_palette]] 開始建立。
2. 輸入其 [[dialog.name]]。
3. 按一下色彩以選取它；[[dialog.add_color]] 會加入目前的色彩，[[dialog.remove_color]] 會移除選取的色彩。
4. 按一下 [[dialog.save_palette]] 將其儲存為檔案。
5. 按一下 [[dialog.close]]。

內建調色盤無法編輯；請建立新的調色盤，或編輯其複本。
