# 透明度

透明度を使うと、オブジェクトの下にあるものが透けて見えます。全体で一様にすることも、グラデーションのようにオブジェクト上で変化させることも、パターンやテクスチャに従わせることもできます。

## 透明度を適用する {#apply}

- [[tool.transparency]] ツールで: オブジェクトをクリックし、プロパティバーで透明度の種類を選びます。またはオブジェクトの上をドラッグすると、不透明 (押した位置) から透明 (離した位置) へのグラデーションの透明度になります。グラデーションのノードは [[tool.interactive_fill]] ツールと同じようにオブジェクト上に表示されます。始点、終点、丸い傾斜ノードをドラッグし、2 つのノードの間のバー (中間点) をドラッグすると中間の位置が変わります。線をダブルクリックするとノードが追加され、ノードをダブルクリックすると削除されます。
- [[docker.properties]] パネルの [[docker.transparency]] セクションで。

## 設定 {#settings}

- 種類: [[transparency.none]]、[[transparency.uniform]] (最初の設定)、[[transparency.fountain]] (グラデーション塗りつぶしと同じく線形、放射、円錐、正方形、シェイプ)、[[transparency.pattern]]、[[transparency.texture]]。
- 量: 0 から 100% (最初は 50%)。グラデーションでは、ストップはグレーの濃淡です: 白が不透明、黒が完全に透明です。
- [[transparency.merge_mode]]: 色を下にあるものとどう混ぜるか。標準 (最初の設定) は単に透けて見えます。そのほかのモード、たとえば加算、減算、差の絶対値、乗算、除算、明るい場合、暗い場合、色相、彩度、カラー、反転は、別の方法で色を組み合わせます。
- [[transparency.target_fill]]、[[transparency.target_outline]]、[[transparency.target_all]] のどれに適用するか。
- **固定**: 下のオブジェクトが動いても、透けて見えるものを今の状態のまま保ちます。

透明度を取り除くには、プロパティバーの最後にある [[toolbar.clear_transparency]] をクリックします（または種類で [[transparency.none]] を選びます）。

別のオブジェクトの透明度をコピーするには、[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]] を使います。オブジェクトの明るさで透けて見える領域を決めるには、[不透明マスク](../objects/clip-frames.md#opacity-mask) を参照してください。
