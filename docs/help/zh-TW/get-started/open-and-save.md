# 開啟和儲存繪圖

> *機器翻譯：本頁可能落後於英文版。*

TraceDraw 以自己的格式 `.tdraw` 儲存繪圖，這種格式會保留所有內容；它也可以儲存 `.cdr` 檔案。它還能開啟許多其他格式；請參閱[支援的檔案格式](../file-formats/supported-formats.md)。

## 開啟繪圖 {#open}

1. 選擇 [[menu.file > menu.file.open]]（<kbd>Ctrl+O</kbd>），或按一下標準工具列上的「開啟」。
2. 選擇檔案並確認。

繪圖會在新的分頁中開啟。若要開啟最近使用過的繪圖，請使用 [[menu.file > menu.file.open_recent]]、「開啟」按鈕的箭頭，或歡迎畫面上的清單；[[menu.file.clear_recent]] 會清空清單。若只要從歡迎畫面的清單移除一個繪圖，請指向它並按一下右側的 x，或在它上面按右鍵並選擇 [[welcome.remove_recent]]；檔案本身會保留在原處。[[menu.file > menu.file.import]]（<kbd>Ctrl+I</kbd>）則是將檔案的內容加入目前開啟的繪圖；請參閱[匯入檔案](../file-formats/import.md)。

您也可以從檔案管理員將檔案拖曳到 TraceDraw。繪圖（例如 `.tdraw` 或 `.cdr` 檔案）放在標題列、功能表、工具列或文件分頁上時，會在自己的分頁中開啟；放在繪圖視窗上時，會匯入目前開啟的繪圖。拖曳時顯示的外框和標籤會說明將發生什麼事。在系統不告知指標位置的環境（Linux、macOS）中，繪圖會在分頁中開啟，圖片則會匯入。

繪圖最初如何顯示（整個頁面、其物件、頁面寬度、實際大小，或儲存時的頁面、位置和縮放）由 [[menu.tools > menu.tools.options > menu.tools.options_app]] 的 [[options.display]] 頁面上的 [[options.open_view]] 設定。

### 開啟後的通知 {#open-notes}

當開啟的檔案中有些物件無法讀取，或繪圖使用了電腦上未安裝的字型時，檔案開啟後會有一則訊息告知：

- 對於無法讀取的物件，訊息會列出其數量。**回報**按鈕會在專案的問題追蹤系統中為該檔案開啟一個新的問題，以便修正問題。
- 對於缺少的字型，[[missing_fonts.title]] 對話方塊會列出每個字型以及目前繪製它的已安裝字型，並讓您選擇只在本次工作階段使用另一個字型，或在繪圖中取代它。請參閱[缺少的字型](../text/missing-fonts.md)。

勾選**不要再顯示此訊息**即可停止顯示這些訊息。您可以在 [[menu.tools > menu.tools.options > menu.tools.options_global]] 的 [[options.warnings]] 頁面上再次開啟它們。

## 儲存繪圖 {#save}

選擇 [[menu.file > menu.file.save]]（<kbd>Ctrl+S</kbd>）可將繪圖寫回其檔案。新的繪圖，或是從 TraceDraw 只能讀取的格式開啟的繪圖，還沒有檔案，因此「儲存」會開啟 [[dialog.save_drawing]] 對話方塊。

## 另存新檔 {#save-as}

[[menu.file > menu.file.save_as]]（<kbd>Ctrl+Shift+S</kbd>）會以其他名稱、在其他資料夾或以其他格式儲存繪圖。

![儲存繪圖對話方塊](../../img/get-started-open-and-save-save-as.png)

以新名稱儲存繪圖：

1. 選擇資料夾。左側的清單有 [[dialog.place_home]]、[[dialog.place_desktop]]、[[dialog.place_documents]]、[[dialog.place_downloads]] 和 [[dialog.place_pictures]] 以及各磁碟機的捷徑。[[dialog.up_folder]] 會移至上一層資料夾；您也可以在路徑列中輸入資料夾。按兩下清單中的資料夾即可開啟；按一下檔案即可使用其名稱。
2. 輸入 [[dialog.file_name]]。所選類型的副檔名會自動加上。
3. 選擇 [[dialog.save_as_type]]：TraceDraw（`*.tdraw`）或 [[file.cdr_files]]。
4. 選擇 [[dialog.cdr_version]]。對於 TraceDraw 檔案，這是格式的版本，最新的在最前面。對於 `.cdr` 檔案，這是檔案的版本，從 27 到 7；較舊的程式只能開啟與其本身同樣舊或更舊的版本。您選擇的版本會隨繪圖保留，因此「儲存」會直接再次以該版本寫入，不會詢問。
5. 選擇要和繪圖一起放入檔案的內容：
   - 對於版本 16 以上的 `.cdr` 檔案，[[dialog.embed_color_profile_none]] 會將繪圖的 CMYK 描述檔儲存在檔案中。
   - [[dialog.embed_fonts]]（預設關閉）會在授權允許時，將文字使用的字型儲存在 TraceDraw 檔案和版本 16 以上的 `.cdr` 檔案中。核取方塊會顯示字型增加的檔案大小。要交給可能沒有這些字型的電腦時，請勾選它。
   - 在桌面版的 TraceDraw 檔案中，[[dialog.save_package]] 會將繪圖連同其連結的圖片、字型檔和一份報告儲存在以繪圖命名的資料夾中，[[package.pdf]] 則會加入一份 PDF。請參閱[封裝繪圖](../file-formats/package.md)。

   這些選擇都會保留到下一次。
6. 視需要填寫 [[dialog.title]]、[[dialog.subject]] 和 [[dialog.rating]]（再按一下亮起的最後一顆星即可取消）。這些是繪圖的屬性，也會顯示在[文件屬性](../pages-layout/document-properties.md)中。
7. 按一下 [[dialog.save]] 或按 <kbd>Enter</kbd>。如果檔案已存在，TraceDraw 會詢問是否要 [[dialog.replace]]。

對話方塊會從繪圖所在的資料夾開始；若沒有，則從您上次儲存的資料夾開始；再沒有則從您的文件資料夾開始。從 `.cdr` 檔案或其他格式開啟的繪圖，會從該檔案所在的資料夾並以其名稱開始，因此以相同格式儲存會取代它。

在檔案清單中，[[dialog.new_folder]] 會建立資料夾並讓您立即輸入名稱；<kbd>F2</kbd> 或按右鍵可重新命名資料夾或檔案；輸入字母會跳到以這些字母開頭的項目，<kbd>Enter</kbd> 會開啟所選的資料夾。在路徑列中輸入的資料夾不必按 <kbd>Enter</kbd> 即會採用。

> **注意：** 在瀏覽器版本中，每次儲存都是一次下載：對話方塊會保留名稱、類型、版本和屬性，而瀏覽器會將檔案放在其下載資料夾中。

其他儲存方式：[[menu.file > menu.file.save_as_template]] 會儲存範本（請參閱[範本](../templates-styles/templates.md)），[[menu.file.export]] 會寫入其他格式（請參閱[匯出](../file-formats/export.md)），[[menu.file.publish_to_pdf]] 會寫入 PDF（請參閱[發佈為 PDF](../file-formats/publish-pdf.md)），[[menu.file.send_to]] 會將 PDF 儲存到您的桌面或文件資料夾，或將其附加到電子郵件中，[[menu.file.package]] 會將繪圖、其圖片和字型放入一個資料夾（請參閱[封裝繪圖](../file-formats/package.md)），而 [[menu.file.github]] 會將繪圖保存在 GitHub 儲存庫中（請參閱 [GitHub](github.md)）。[[menu.file.revert]] 會回到上次儲存的版本。

## 關閉繪圖 {#closing}

選擇 [[menu.file > menu.file.close]]（<kbd>Ctrl+F4</kbd>）、按一下繪圖分頁上的關閉按鈕，或以滑鼠中鍵按一下分頁。[[menu.file.close_all]] 會關閉所有繪圖，[[menu.file.exit]]（<kbd>Alt+F4</kbd>）則會關閉 TraceDraw。

當繪圖有未儲存的變更時，TraceDraw 會先詢問是否要儲存：[[dialog.yes]] 會儲存（若繪圖還沒有檔案，會要求輸入名稱），[[dialog.no]] 會不儲存就關閉，[[dialog.cancel]] 則會保持繪圖開啟。當多個繪圖有變更時，它會逐一詢問；按「取消」即停止。

![關於未儲存變更的詢問](../../img/get-started-open-and-save-closing.png)

## 自動備份與復原 {#recovery}

在桌面版中，當自動備份開啟時，TraceDraw 會為您開啟的繪圖保留備份（[[menu.tools > menu.tools.options > menu.tools.options_app]] 的 [[options.save]] 頁面可設定頻率和位置；預設為每 20 分鐘一次）。正常關閉 TraceDraw 時會移除這些備份。

如果 TraceDraw 未正常關閉就結束了，例如停電後，下次啟動時它會提供這些備份：[[dialog.recover]] 會將每個備份以未儲存的繪圖開啟（之後「儲存」會詢問要存到哪裡），[[dialog.discard]] 則會將備份留在其資料夾中。同時執行的兩個 TraceDraw 視窗永遠不會提供彼此的繪圖。
