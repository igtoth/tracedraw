# 図面を開く、保存する

TraceDraw は、すべてを保持する独自の形式 `.tdraw` で図面を保存し、`.cdr` ファイルとしても保存できます。ほかにも多くの形式を開けます。[対応しているファイル形式](../file-formats/supported-formats.md) を参照してください。

## 図面を開く {#open}

1. [[menu.file > menu.file.open]] (<kbd>Ctrl+O</kbd>) を選ぶか、標準ツールバーの「開く」をクリックします。
2. ファイルを選んで確定します。

図面は新しいタブで開きます。最近使った図面を開くには、[[menu.file > menu.file.open_recent]]、「開く」ボタンの矢印、またはようこそ画面の一覧を使います。[[menu.file.clear_recent]] は一覧を空にします。ようこそ画面の一覧から図面を1つだけ外すには、その図面を指して右端の x をクリックするか、右クリックして [[welcome.remove_recent]] を選びます。ファイル自体はそのまま残ります。一方、[[menu.file > menu.file.import]] (<kbd>Ctrl+I</kbd>) は、ファイルの内容を開いている図面に追加します。[ファイルをインポートする](../file-formats/import.md) を参照してください。

### 開いた後のお知らせ {#open-notes}

開いたファイルの一部のオブジェクトを読み込めなかった場合や、図面がコンピューターにインストールされていないフォントを使っている場合は、ファイルが開いた後にメッセージで知らせます:

- 読み込めなかったオブジェクトについては、その数を示します。**報告** ボタンは、プロジェクトの issue トラッカーにそのファイルの新しい issue を作成し、問題を修正できるようにします。
- 見つからないフォントについては、その名前を示します。テキストは似たフォントで表示され、フォント名は保たれるので、そのフォントがあるコンピューターでは元どおりに戻ります。

**今後このメッセージを表示しない** にチェックを付けると、これらのメッセージは表示されなくなります。[[menu.tools > menu.tools.options > menu.tools.options_global]] の [[options.warnings]] ページで再びオンにできます。

## 図面を保存する {#save}

[[menu.file > menu.file.save]] (<kbd>Ctrl+S</kbd>) を選ぶと、図面をそのファイルに書き戻します。新しい図面や、TraceDraw が読み込みしかできない形式から開いた図面にはまだファイルがないので、保存すると [[dialog.save_drawing]] ダイアログボックスが開きます。

## 名前を付けて保存 {#save-as}

[[menu.file > menu.file.save_as]] (<kbd>Ctrl+Shift+S</kbd>) は、図面を別の名前、別のフォルダー、または別の形式で保存します。

![図面の保存ダイアログボックス](../../img/get-started-open-and-save-save-as.png)

図面を新しい名前で保存するには:

1. フォルダーを選びます。左側の一覧には [[dialog.place_home]]、[[dialog.place_desktop]]、[[dialog.place_documents]]、[[dialog.place_downloads]]、[[dialog.place_pictures]] とドライブへのショートカットがあります。[[dialog.up_folder]] は一つ上のフォルダーに移動します。パスバーにフォルダーを入力して <kbd>Enter</kbd> を押すこともできます。[[dialog.new_folder]] はフォルダーを作ってそれを開きます。一覧のフォルダーをダブルクリックすると開き、ファイルをクリックするとその名前を使います。
2. [[dialog.file_name]] を入力します。選んだ種類の拡張子が付きます。
3. [[dialog.save_as_type]] を選びます: TraceDraw (`*.tdraw`) または [[file.cdr_files]]。
4. [[dialog.cdr_version]] を選びます。TraceDraw ファイルでは形式のバージョンで、新しい順に並びます。`.cdr` ファイルではファイルのバージョンで、27 から 14 までです。古いプログラムは、自分と同じかそれより古いバージョンしか開けません。選んだバージョンは図面に記憶されるので、次の保存では尋ねずにそのバージョンで書き込みます。
5. バージョン 16 以降の `.cdr` ファイルでは、[[dialog.embed_color_profile_none]] は図面の CMYK プロファイルをファイルに格納し、[[dialog.embed_fonts]] はライセンスが許す場合にテキストで使うフォントを格納します。どちらの選択も次回のために記憶されます。
6. 必要に応じて [[dialog.title]]、[[dialog.subject]]、[[dialog.rating]] を入力します (点灯している最後の星をもう一度クリックすると外れます)。これらは図面のプロパティで、[ドキュメントのプロパティ](../pages-layout/document-properties.md) にも表示されます。
7. [[dialog.save]] をクリックするか <kbd>Enter</kbd> を押します。ファイルが既にある場合は、[[dialog.replace]] するかどうかを TraceDraw が尋ねます。

ダイアログボックスは、図面のフォルダー、なければ最後に保存したフォルダー、それもなければドキュメントフォルダーで開きます。

> **メモ:** ブラウザー版では、保存はすべてダウンロードになります。ダイアログボックスは名前、種類、バージョン、プロパティを保持し、ブラウザーがファイルをダウンロードフォルダーに置きます。

そのほかの保存方法: [[menu.file > menu.file.save_as_template]] はテンプレートを保存します ([テンプレート](../templates-styles/templates.md) を参照)。[[menu.file.export]] はほかの形式で書き出します ([エクスポート](../file-formats/export.md) を参照)。[[menu.file.publish_to_pdf]] は PDF を書き出します ([PDF に発行する](../file-formats/publish-pdf.md) を参照)。[[menu.file.send_to]] は PDF をデスクトップかドキュメントフォルダーに保存するか、メールに添付します。[[menu.file.github]] は図面を GitHub のリポジトリに保管します ([GitHub](github.md) を参照)。[[menu.file.revert]] は最後に保存したバージョンに戻します。

## 図面を閉じる {#closing}

[[menu.file > menu.file.close]] (<kbd>Ctrl+F4</kbd>) を選ぶか、図面のタブの閉じるボタンをクリックするか、タブを中クリックします。[[menu.file.close_all]] はすべての図面を閉じ、[[menu.file.exit]] (<kbd>Alt+F4</kbd>) は TraceDraw を終了します。

図面に保存していない変更があると、TraceDraw は先に保存するかどうかを尋ねます: [[dialog.yes]] は保存し (図面にまだファイルがなければ名前を尋ねます)、[[dialog.no]] は保存せずに閉じ、[[dialog.cancel]] は図面を開いたままにします。複数の図面に変更がある場合は、一つずつ尋ねます。キャンセルするとそこで止まります。

![保存していない変更についての確認](../../img/get-started-open-and-save-closing.png)

## 自動バックアップと復元 {#recovery}

デスクトップでは、自動バックアップがオンの間、TraceDraw は開いている図面のバックアップを保持します ([[menu.tools > menu.tools.options > menu.tools.options_app]] の [[options.save]] ページで頻度と場所を設定します。最初は 20 分ごとです)。TraceDraw を通常どおり終了すると、バックアップは削除されます。

停電などで TraceDraw が正常に終了しなかった場合は、次に起動したときにバックアップを提示します: [[dialog.recover]] はそれぞれを保存されていない図面として開き (保存すると保存場所を尋ねます)、[[dialog.discard]] はバックアップをフォルダーに残します。同時に動いている二つの TraceDraw ウィンドウが、互いの図面を提示することはありません。
