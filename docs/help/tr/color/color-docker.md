# Renk paneli

[[docker.color]] paneli (<kbd>Shift+F11</kbd>) bir renk karıştırır ve onu
seçimin dolgusuna veya anahattına uygular. [[tool.outline_color]] aracı
(<kbd>Shift+F12</kbd>) da onu açar. Paneli
[[menu.window > menu.window.dockers]] menüsünden açın.

![Renk paneli](../../img/color-color-docker.png)

## Renk seçme {#choose}

1. Nesneleri seçin. Panel ilk nesnenin dolgu renginden başlar.
2. Yeni rengi görünümlerden birinde karıştırın:
   - [[docker.color_viewers]]: doygunluğu (soldan sağa) ve parlaklığı (alttan
     üste) ayarlamak için karede, tonu ayarlamak için yanındaki şeritte
     tıklayın veya sürükleyin;
   - [[docker.color_sliders]]: [[docker.model]] öğesinin her bileşeni için bir
     kaydırıcı: RGB, CMYK (başlangıçta), HSB, HSL, Lab, Gri tonlama veya YIQ;
   - [[docker.color_palettes]]: bir paletin renk örnekleri.

   Üstteki üç düğme her görünümü gösterir veya gizler; herhangi bir
   birleşim gösterilebilir. Bir `#rrggbb` değeri de yazabilirsiniz.
3. Yeni rengi uygulamak için panelin altındaki [[docker.fill]] veya
   [[docker.outline]] düğmesine tıklayın.
   Kilit ([[docker.lock_colors]]) dolgu ile anahatı birbirine bağlar; böylece
   iki düğmeden biri rengi her ikisine de uygular. "..." düğmesi
   ([[docker.more_actions]]) diğer eylemleri açar: [[docker.no_fill]],
   [[docker.no_outline_action]], [[docker.swap_colors]],
   [[docker.add_to_document_palette]], [[docker.add_to_spot_colors]] ve
   [[docker.add_to_color_styles]].

Eski renk ile yenisi yan yana görünür. Yeni renk CMYK mürekkepleriyle tam
olarak basılamıyorsa panel bir uyarı gösterir.

## Renk örnekleme {#sample}

[[docker.eyedropper]] düğmesine tıklayın ve oradaki rengi almak için çizimde
herhangi bir yere tıklayın. Ardından onu yeni renk yapmak için
[[docker.use_sampled]] düğmesine tıklayın. Ayrıca bkz.
[Damlalıklar](eyedroppers.md).

## Spot mürekkepler {#spot-inks}

Düğmelerin altında [[docker.spot_inks]] çizimin spot mürekkeplerini listeler;
bkz. [Spot renkler](spot-colors.md).
