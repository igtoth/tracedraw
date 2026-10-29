# カラーパネル

[[docker.color]] パネル (<kbd>Shift+F11</kbd>) は色を調合し、選択範囲の塗りつぶしまたはアウトラインに適用します。[[tool.outline_color]] ツール (<kbd>Shift+F12</kbd>) でも開きます。[[menu.window > menu.window.dockers]] から開きます。

![カラーパネル](../../img/color-color-docker.png)

## 色を選ぶ {#choose}

1. オブジェクトを選択します。パネルは最初のオブジェクトの塗りつぶしの色から始まります。
2. いずれかの表示で新しい色を調合します:
   - [[docker.color_viewers]]: 正方形の中をクリックまたはドラッグして彩度 (左から右) と明るさ (下から上) を設定し、横の帯で色相を設定します。
   - [[docker.color_sliders]]: [[docker.model]] の各成分に一つずつスライダーがあります。RGB、CMYK (最初の設定)、HSB、HSL、Lab、グレースケール、YIQ です。
   - [[docker.color_palettes]]: パレットの色見本。

   上部の三つのボタンで、各表示を表示または非表示にします。どの組み合わせでも表示できます。`#rrggbb` の値を入力することもできます。
3. パネル下部の [[docker.fill]] または [[docker.outline]] をクリックすると、新しい色を適用します。ロック（[[docker.lock_colors]]）は塗りとアウトラインを結び付け、どちらのボタンでも色を両方に適用します。「...」ボタン（[[docker.more_actions]]）はその他の操作を開きます: [[docker.no_fill]], [[docker.no_outline_action]], [[docker.swap_colors]], [[docker.add_to_document_palette]], [[docker.add_to_spot_colors]]、[[docker.add_to_color_styles]]。

古い色と新しい色が並んで表示されます。新しい色を CMYK インキで正確に印刷できない場合は、パネルに警告が表示されます。

## 色をサンプリングする {#sample}

[[docker.eyedropper]] をクリックし、図面のどこかをクリックすると、その位置の色を取得します。続いて [[docker.use_sampled]] をクリックすると、それが新しい色になります。[スポイト](eyedroppers.md) も参照してください。

## 特色インキ {#spot-inks}

ボタンの下の [[docker.spot_inks]] には、図面の特色インキが一覧されます。[特色](spot-colors.md) を参照してください。
