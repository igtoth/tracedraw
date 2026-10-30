# ビットマップのトレース

トレースは、ビットマップを編集、色の変更、拡大縮小しても品質が落ちないベクターの図形に変換します。スキャンしたロゴ、スケッチ、写真などに使います。

## クイックトレース {#quick-trace}

ビットマップを選択し、[[menu.bitmaps > menu.bitmaps.quick_trace]] を選びます。グローバルオプションの [[options.bitmap_tracing]] ページで選んだプリセット (最初はクリップアート) を使います。

[[menu.bitmaps.repeat_last_trace]] は選択したビットマップをダイアログを開かずに前回と同じ設定でトレースし、[[menu.bitmaps.last_trace_settings]] は前回の設定でダイアログボックスを開きます。ビットマップの右クリックメニューにも、[[menu.bitmaps.centerline_trace]] と [[menu.bitmaps.outline_trace]] とともにこれらのコマンドがあります。

## ビットマップのトレースダイアログボックス {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] と [[menu.bitmaps.centerline_trace]] にはプリセットが一覧されています。一つを選ぶとトレースのダイアログボックスが開きます。

- **アウトライントレース** は、色の領域を図形で塗りつぶします: [[menu.bitmaps.trace_line_art]]、[[menu.bitmaps.trace_logo]]、[[menu.bitmaps.trace_detailed_logo]]、[[menu.bitmaps.trace_clipart]]、[[menu.bitmaps.trace_low_quality]]、[[menu.bitmaps.trace_high_quality]]。
- **センターライントレース** は、線を開いたカーブでなぞります: [[menu.bitmaps.trace_technical]] と [[menu.bitmaps.trace_line_drawing]]。

![ビットマップのトレースダイアログボックス](../../img/bitmaps-tracing.png)

このダイアログボックスは、サイズを変更したり最大化したりできるウィンドウです。プレビューが左側を占め、設定は右側のパネルにあり、設定を変えるたびにトレースが作り直されます: 結果を見るために押すものはありません。プレビューの下の行には、トレースの **カーブ**、**ノード**、**色** の数が表示されます。[[dialog.ok]] をクリックすると図形がビットマップの上に暗いものから順に置かれ、[[dialog.cancel]] をクリックするとビットマップはそのままになります。

## プレビュー {#preview}

[[dialog.trace_preview_label]] リストで、プレビューに表示するものを選びます:

- [[dialog.trace_view_before_after]]: 上にビットマップ、下にそのトレース。または [[dialog.view_side_by_side]]。[[dialog.before]] はビットマップだけを表示します。
- [[dialog.trace_view_large]]: トレースだけを、プレビューいっぱいの大きさで表示します。
- [[dialog.trace_view_wireframe]]: トレースのカーブを細い線でビットマップの上に描きます。リストの横に [[dialog.trace_overlay]] スライダーが現れます: 線の下でビットマップをどれだけ強く見せるかを、0 (線だけ) から 100 (ビットマップそのまま) で設定します。最初は 80 です。

リストの後ろのボタンは、全体表示、縮小、拡大、パンです。ホイールはポインターを中心に拡大縮小し、ドラッグで画像が動きます。2 つのペインは一緒に動きます。透明な部分は市松模様で表示されます。

## 設定 {#settings}

[[dialog.trace_tab_settings]] タブには次があります:

- [[dialog.trace_type]] のアウトラインまたはセンターライン、および [[dialog.trace_image_type]] のプリセット。プリセットを選ぶと、下のスライダーがその種類の画像に合う値になります。
- [[dialog.detail]] (0 から 100): 詳細を上げると、より小さな領域と細い線まで残します。下げると、しみや粒子が滑らかになります。
- [[dialog.smoothing]] (0 から 100、最初は 25): ノードが少なく、より滑らかなカーブになります。
- [[dialog.corner_smoothness]] (0 から 100): 角が丸くなります。
- [[dialog.trace_remove]]: トレースから除くもの。**背景色** は画像の縁に達している背景を取り除き、**画像から色全体** はその色の領域をすべて取り除き、**なし** はすべてをトレースします。[[dialog.trace_remove_auto]] は画像の縁に沿って背景色を見つけます。[[dialog.trace_remove_specify]] はスウォッチの色を使います。この色は変更したり、横のスポイトでプレビューから取ったりできます。
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (最初はオン) は、同じ色の領域すべてを 1 本のカーブにします。オフにすると、離れた部分はそれぞれ別のカーブになります。[[dialog.trace_remove_overlap]] は、各図形を自分のピクセルだけにとどめます。オフにすると、図形は上の図形の下に少し入り込み、隣り合う図形の間に隙間ができません。[[dialog.trace_group_by_color]] は、各色のカーブをそれぞれのグループに入れます。同じ色の領域がまだ結合されていないときに使えます。
- [[dialog.delete_original]]: トレースを置くときにビットマップを削除します。

現在のトレースの種類に関係のない設定は灰色で表示されます: センターライントレースは、詳細とスムージングだけを読みます。

## 色 {#colors}

[[dialog.trace_tab_colors]] タブでは、[[dialog.color_mode]] (RGB、グレースケール、または白黒) と [[dialog.trace_number_of_colors]] (2 から 256) を設定し、トレースの色をそれぞれのカーブ数とともに一覧します。2 つ以上の色にチェックを付けて [[dialog.trace_merge_colors]] をクリックすると、最初の色にまとめられます。色のスウォッチをクリックすると、そのカーブを別の色で描けます。画像をトレースし直す設定を変更すると、前のトレースのものだったこれらの編集は忘れられます。

## リセット、元に戻す、やり直す {#steps}

設定パネルの下の [[dialog.reset]] はプリセットの設定に戻し、2 つの矢印はダイアログボックスで行ったすべての変更を後ろと前にたどります: スライダーを放すたび、リストの選択、チェックボックス、色の編集、リセットのそれぞれが 1 ステップです。元に戻した後に変更すると、元に戻したステップは捨てられます。

[[dialog.trace_options]] はグローバルオプションの [[options.bitmap_tracing]] ページを開き、クイックトレースのプリセットと、すべてのトレースの最初の選択を選んだ後、ダイアログボックスに戻ります。[[dialog.help_button]] ボタンはこのページを開きます。
