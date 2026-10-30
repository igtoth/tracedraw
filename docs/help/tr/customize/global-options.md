# Genel seçenekler

> *Makine çevirisi: bu sayfa İngilizce sürümün gerisinde kalabilir.*

[[menu.tools > menu.tools.options > menu.tools.options_global]] programın
tamamına uygulanan ayarları barındırır: dil, yazdırma ve dışa aktarma
varsayılanları, izleme, iletiler ve dosyaların nerede tutulduğu.

## Genel {#general}

![Genel seçeneklerin Genel sayfası](../../img/customize-global-options-general.png)

- [[options.language]]: arayüzün dili. Bu Yardım da aynı dilde açılır.
- [[options.check_updates]] (açık): bkz.
  [Güncellemeleri denetleme](../welcome/about-and-updates.md#updates).
- [[options.agents]]: [[options.mcp_window]], bu bilgisayardaki YZ
  asistanlarının açık çizimi düzenlemesine izin verir; bkz.
  [Betikler](scripts.md#mcp).
- [[options.reset_all]]: bkz. [Ayarları sıfırlama](reset-settings.md).

## Yazdırma {#printing}

![Genel seçeneklerin Yazdırma sayfası](../../img/customize-global-options-printing.png)

[[options.print_starts_with]]: [[dialog.fit_to_page]],
[[dialog.separations]], [[dialog.print_bleed]], [[dialog.crop_marks]],
[[dialog.registration_marks]], [[print.color_bars]], [[print.densitometer]],
[[print.page_info_mark]] (hepsi kapalı), [[print.sheet]],
[[print.landscape]] ve [[print.gutter]] (5 mm).
[[options.print_remember]] son yazdırmanın seçimlerini tutar.
[[options.print_checks]] altında [[options.print_preflight]] (açık). Bkz.
[Bir çizimi yazdırma](../printing/print.md).

## Dosya Biçimleri {#file-formats}

![Genel seçeneklerin Dosya Biçimleri sayfası](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]]: [[dialog.format]] (SVG), bitmap
  biçimlerinin [[dialog.resolution]] değeri (300 dpi), JPEG ve WebP'nin
  [[dialog.quality]] değeri (90), [[dialog.transparent_background]] (açık),
  [[dialog.pdf_standard]], [[dialog.output_condition]] ve [[dialog.bleed]].
  [[options.export_remember]] son dışa aktarmanın seçimlerini tutar.
- [[options.import_section]]: [[options.import_place]]
  [[options.import_as_in_file]] (başlangıçta) veya
  [[options.import_view_center]].
- [[options.cdr_save_section]]: yeni çizimlerin `.cdr` olarak kaydedileceği
  [[dialog.cdr_version]].

## Bitmap İzleme {#bitmap-tracing}

![Genel seçeneklerin Bitmap İzleme sayfası](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]]: ön ayarlardan biri (başlangıçta Küçük Resim).
[[options.trace_starts_with]]: [[dialog.remove_background]] (açık) ve
[[dialog.delete_original]] (kapalı). Bkz.
[Bitmapleri izleme](../bitmaps/tracing.md).

## Uyarılar {#warnings}

![Genel seçeneklerin Uyarılar sayfası](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]],
[[options.warn_missing_fonts]] ve [[options.warn_shelf_remove]] (hepsi
açık). Her iletide onu durduran bir kutu da vardır; yeniden görmek için
seçeneği burada işaretleyin.

[[options.ask_missing_fonts]] (açık), yüklü olmayan yazı tipleri kullanan
bir çizim açıldığında Eksik Yazı Tipleri iletişim kutusunu gösterir; bkz.
[Eksik yazı tipleri](../text/missing-fonts.md). Yukarıdaki basit iletiden
ayrıdır.

## Dosya Konumları {#file-locations}

![Genel seçeneklerin Dosya Konumları sayfası](../../img/customize-global-options-file-locations.png)

Tercihlerin ve otomatik yedeklerin nerede tutulduğunu gösterir. Tercihler,
varsa programın yanındaki bir `tracedraw-settings` klasöründe (taşınabilir
bir kopya), yoksa sisteminizin program ayarları için ayırdığı yerdedir;
tarayıcı sürümü onları tarayıcıda tutar.
