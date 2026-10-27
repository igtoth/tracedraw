# TraceDraw seçenekleri

[[menu.tools > menu.tools.options > menu.tools.options_app]]
(<kbd>Ctrl+J</kbd>) programın nasıl çalışacağını ayarlar. Soldaki listeden
bir sayfa seçin.

## Genel {#general}

![TraceDraw seçeneklerinin Genel sayfası](../../img/customize-options-app-general.png)

- [[options.on_startup]]: Karşılama Ekranını göster, yeni bir belge başlat
  veya son düzenlenen belgeyi aç.
- [[options.show_new_document_dialog]] (açık): yeni bir çizimin ayarlarını
  sor; bkz. [Çizim oluşturma](../get-started/new-document.md).
- [[options.show_splash]] (açık).
- [[options.hardware_acceleration]] (açık): pencere yanında seçilen ekran kartıyla çizilir (başta [[options.gpu_automatic]]); kapalıyken bilgisayarın yazılım oluşturucusuyla. Bir sonraki açılışta geçerli olur. Pencere böyle açılamazsa TraceDraw yazılım oluşturucuyla, ardından OpenGL ile yeniden başlar ve çalışanı hatırlar.
- [[options.undo_levels]]: kaç adımı geri alabileceğiniz (başlangıçta 150).

## Ekran {#display}

![TraceDraw seçeneklerinin Ekran sayfası](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: renkleri basılacakları gibi göster.
- [[options.show_tooltips]] (açık), [[options.hide_bbox_curve_tools]] ve
  [[options.node_tracking]] (kapalı: açıkken [[tool.pick]] aracı seçili bir
  eğrinin düğümlerini sürükleyebilir).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (başlangıçta)
  veya [[options.wheel_scroll]].
- [[options.ruler_calibration]]: ekrana bir cetvel tutun ve işaretler
  eşleşene kadar [[options.screen_resolution]] değerini değiştirin; böylece
  %100 yakınlaştırma gerçek boyutları gösterir. [[options.reset]] 96 değerine
  geri döndürür.

## Düzen {#edit}

![TraceDraw seçeneklerinin Düzen sayfası](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: döndürürken ve çizerken <kbd>Ctrl</kbd>
  tuşunun adımı (başlangıçta 15 derece).
- [[options.drawing_precision]]: özellik çubuğundaki ve durum çubuğundaki
  ondalık basamak sayısı (başlangıçta 3).
- [[options.focus_inactive]] (başlangıçta %20) ve
  [[options.focus_double_click]]; bkz. [Odak modu](../objects/focus-mode.md).

## Düğümler ve Tutamaçlar {#nodes-and-handles}

![TraceDraw seçeneklerinin Düğümler ve Tutamaçlar sayfası](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: küçük, orta veya büyük.
- [[options.node_cusp]], [[options.node_smooth]] ve
  [[options.node_symmetrical]] düğümler için [[options.node_shape]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: seçili düğümlerin ve tutamaçların
  [[options.main_color]] öğesi ve [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Kırpma Çerçevesi {#clip-frame}

![TraceDraw seçeneklerinin Kırpma Çerçevesi sayfası](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (başlangıçta),
  [[options.auto_center_always]] veya [[options.auto_center_never]].
- [[options.empty_frame_lines]] (açık).

Bkz. [Nesneleri bir çerçevenin içine yerleştirme](../objects/clip-frames.md).

## Yakalama {#snapping}

![TraceDraw seçeneklerinin Yakalama sayfası](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], taban çizgisi
  ızgarası ve [[options.snap_to_pixels]].
- [[options.snapping_radius]] (başlangıçta 10 piksel).
- [[options.show_snap_marks]] ve [[options.snap_screen_tip]].
- [[options.snap_modes]]: nesnelerin yakalandığı noktalar,
  [[options.select_all]] ve [[options.deselect_all]] ile birlikte.

Bkz. [Yakalama](../pages-layout/snapping.md).

## Kaydet {#save}

![TraceDraw seçeneklerinin Kaydet sayfası](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (açık) önceki dosyayı
  `backup_of_<name>` olarak tutar; [[options.backup_to]]
  [[options.same_folder]] veya seçilen bir yer.
- [[options.auto_backup]] (açık): [[options.backup_every]] 20
  [[options.minutes]], geçici bir klasöre veya seçilen bir yere. Bir
  çökmeden sonra yedekler önerilir; bkz.
  [Otomatik yedekleme ve kurtarma](../get-started/open-and-save.md#recovery).

## Metin {#text}

![TraceDraw seçeneklerinin Metin sayfası](../../img/customize-options-app-text.png)

Sayfada beş sekme vardır.

- [[options.display]]: [[options.keyboard_text_increment]] (<kbd>Ctrl+8</kbd>
  ve <kbd>Ctrl+2</kbd> seçili metni ne kadar büyütüp küçültür, 1 punto), yeni
  metnin [[options.default_font]] ve [[options.default_size]] değerleri ve
  [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] ve [[options.hyphenation_patterns]]
  (yerleşik ya da dilinize ait bir heceleme sözlüğü).
- [[docker.fonts]]: yazı tipi listelerinin başında
  [[options.show_recent_fonts]], [[options.recent_font_count]] (5, en çok 20)
  ve [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  ve [[options.add_to_autocorrect]]; yazım denetiminde düzelttiğiniz bir sözcük
  bir dahaki sefere yazarken düzeltilir; ve kullandığı
  [[options.dictionaries]], bkz. [Sözlükler](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: yazarken neyin düzeltildiği; bkz.
  [Otomatik Düzelt](../text/writing-tools.md#autocorrect).
