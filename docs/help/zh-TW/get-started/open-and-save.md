# 開啟和儲存繪圖

TraceDraw 以自己的格式 `.tdraw` 儲存繪圖，這種格式會保留所有內容；它也可以儲存 `.cdr` 檔案。它還能開啟許多其他格式；請參閱[支援的檔案格式](../file-formats/supported-formats.md)。

## 開啟繪圖 {#open}

1. 選擇 [[menu.file > menu.file.open]]（<kbd>Ctrl+O</kbd>），或按一下標準工具列上的「開啟」。
2. 選擇檔案並確認。

繪圖會在新的分頁中開啟。若要開啟最近使用過的繪圖，請使用 [[menu.file > menu.file.open_recent]]、「開啟」按鈕的箭頭，或歡迎畫面上的清單；[[menu.file.clear_recent]] 會清空清單。若只要從歡迎畫面的清單移除一個繪圖，請指向它並按一下右側的 x，或在它上面按右鍵並選擇 [[welcome.remove_recent]]；檔案本身會保留在原處。[[menu.file > menu.file.import]]（<kbd>Ctrl+I</kbd>）則是將檔案的內容加入目前開啟的繪圖；請參閱[匯入檔案](../file-formats/import.md)。

### 開啟後的通知 {#open-notes}

當開啟的檔案中有些物件無法讀取，或繪圖使用了電腦上未安裝的字型時，檔案開啟後會有一則訊息告知：

- 對於無法讀取的物件，訊息會列出其數量。**回報**按鈕會在專案的問題追蹤系統中為該檔案開啟一個新的問題，以便修正問題。
- 對於缺少的字型，訊息會列出其名稱。文字會以類似的字型顯示，並保留其字型名稱，因此在安裝了該字型的電腦上會恢復原狀。

勾選**不要再顯示此訊息**即可停止顯示這些訊息。您可以在 [[menu.tools > menu.tools.options > menu.tools.options_global]] 的 [[options.warnings]] 頁面上再次開啟它們。

## 儲存繪圖 {#save}

選擇 [[menu.file > menu.file.save]]（<kbd>Ctrl+S</kbd>）可將繪圖寫回其檔案。新的繪圖，或是從 TraceDraw 只能讀取的格式開啟的繪圖，還沒有檔案，因此「儲存」會開啟 [[dialog.save_drawing]] 對話方塊。

## 另存新檔 {#save-as}

[[menu.file > menu.file.save_as]]（<kbd>Ctrl+Shift+S</kbd>）會以其他名稱、在其他資料夾或以其他格式儲存繪圖。

![儲存繪圖對話方塊](../../img/get-started-open-and-save-save-as.png)

以新名稱儲存繪圖：

1. 選擇資料夾。左側的清單有 [[dialog.place_home]]、[[dialog.place_desktop]]、[[dialog.place_documents]]、[[dialog.place_downloads]] 和 [[dialog.place_pictures]] 以及各磁碟機的捷徑。[[dialog.up_folder]] 會移至上一層資料夾；您也可以在路徑列中輸入資料夾並按 <kbd>Enter</kbd>。[[dialog.new_folder]] 會建立資料夾並開啟它。按兩下清單中的資料夾即可開啟；按一下檔案即可使用其名稱。
2. 輸入 [[dialog.file_name]]。所選類型的副檔名會自動加上。
3. 選擇 [[dialog.save_as_type]]：TraceDraw（`*.tdraw`）或 [[file.cdr_files]]。
4. 選擇 [[dialog.cdr_version]]。對於 TraceDraw 檔案，這是格式的版本，最新的在最前面。對於 `.cdr` 檔案，這是檔案的版本，從 27 到 14；較舊的程式只能開啟與其本身同樣舊或更舊的版本。您選擇的版本會隨繪圖保留，因此「儲存」會直接再次以該版本寫入，不會詢問。
5. 對於版本 16 以上的 `.cdr` 檔案，[[dialog.embed_color_profile_none]] 會將繪圖的 CMYK 描述檔儲存在檔案中，[[dialog.embed_fonts]] 則會在授權允許時儲存文字使用的字型。這兩個選擇都會保留到下一次。
6. 視需要填寫 [[dialog.title]]、[[dialog.subject]] 和 [[dialog.rating]]（再按一下亮起的最後一顆星即可取消）。這些是繪圖的屬性，也會顯示在[文件屬性](../pages-layout/document-properties.md)中。
7. 按一下 [[dialog.save]] 或按 <kbd>Enter</kbd>。如果檔案已存在，TraceDraw 會詢問是否要 [[dialog.replace]]。

對話方塊會從繪圖所在的資料夾開始；若沒有，則從您上次儲存的資料夾開始；再沒有則從您的文件資料夾開始。

> **注意：** 在瀏覽器版本中，每次儲存都是一次下載：對話方塊會保留名稱、類型、版本和屬性，而瀏覽器會將檔案放在其下載資料夾中。

其他儲存方式：[[menu.file > menu.file.save_as_template]] 會儲存範本（請參閱[範本](../templates-styles/templates.md)），[[menu.file.export]] 會寫入其他格式（請參閱[匯出](../file-formats/export.md)），[[menu.file.publish_to_pdf]] 會寫入 PDF（請參閱[發佈為 PDF](../file-formats/publish-pdf.md)），[[menu.file.send_to]] 會將 PDF 儲存到您的桌面或文件資料夾，或將其附加到電子郵件中，而 [[menu.file.github]] 會將繪圖保存在 GitHub 儲存庫中（請參閱 [GitHub](github.md)）。[[menu.file.revert]] 會回到上次儲存的版本。

## 關閉繪圖 {#closing}

選擇 [[menu.file > menu.file.close]]（<kbd>Ctrl+F4</kbd>）、按一下繪圖分頁上的關閉按鈕，或以滑鼠中鍵按一下分頁。[[menu.file.close_all]] 會關閉所有繪圖，[[menu.file.exit]]（<kbd>Alt+F4</kbd>）則會關閉 TraceDraw。

當繪圖有未儲存的變更時，TraceDraw 會先詢問是否要儲存：[[dialog.yes]] 會儲存（若繪圖還沒有檔案，會要求輸入名稱），[[dialog.no]] 會不儲存就關閉，[[dialog.cancel]] 則會保持繪圖開啟。當多個繪圖有變更時，它會逐一詢問；按「取消」即停止。

![關於未儲存變更的詢問](../../img/get-started-open-and-save-closing.png)

## 自動備份與復原 {#recovery}

在桌面版中，當自動備份開啟時，TraceDraw 會為您開啟的繪圖保留備份（[[menu.tools > menu.tools.options > menu.tools.options_app]] 的 [[options.save]] 頁面可設定頻率和位置；預設為每 20 分鐘一次）。正常關閉 TraceDraw 時會移除這些備份。

如果 TraceDraw 未正常關閉就結束了，例如停電後，下次啟動時它會提供這些備份：[[dialog.recover]] 會將每個備份以未儲存的繪圖開啟（之後「儲存」會詢問要存到哪裡），[[dialog.discard]] 則會將備份留在其資料夾中。同時執行的兩個 TraceDraw 視窗永遠不會提供彼此的繪圖。
