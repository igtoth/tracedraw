# Opsi global

> *Terjemahan mesin: halaman ini mungkin tertinggal dari versi bahasa Inggris.*

[[menu.tools > menu.tools.options > menu.tools.options_global]] berisi
pengaturan yang berlaku untuk seluruh program: bahasa, bawaan pencetakan dan
ekspor, penjiplakan, pesan, dan lokasi penyimpanan berkas.

## Umum {#general}

![Halaman Umum pada opsi global](../../img/customize-global-options-general.png)

- [[options.language]]: bahasa antarmuka. Bantuan ini terbuka dalam bahasa
  yang sama.
- [[options.check_updates]] (aktif): lihat
  [Memeriksa pembaruan](../welcome/about-and-updates.md#updates).
- [[options.agents]]: [[options.mcp_window]] memungkinkan asisten AI di
  komputer ini menyunting gambar yang terbuka; lihat [Skrip](scripts.md#mcp).
- [[options.reset_all]]: lihat [Mengatur ulang pengaturan](reset-settings.md).

## Pencetakan {#printing}

![Halaman Pencetakan pada opsi global](../../img/customize-global-options-printing.png)

[[options.print_starts_with]]: [[dialog.fit_to_page]],
[[dialog.separations]], [[dialog.print_bleed]], [[dialog.crop_marks]],
[[dialog.registration_marks]], [[print.color_bars]], [[print.densitometer]],
[[print.page_info_mark]] (semuanya nonaktif), [[print.sheet]],
[[print.landscape]], dan [[print.gutter]] (5 mm). [[options.print_remember]]
menyimpan pilihan dari pencetakan terakhir. Di bawah
[[options.print_checks]], [[options.print_preflight]] (aktif). Lihat
[Mencetak gambar](../printing/print.md).

## Format Berkas {#file-formats}

![Halaman Format Berkas pada opsi global](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]]: [[dialog.format]] (SVG),
  [[dialog.resolution]] format bitmap (300 dpi), [[dialog.quality]] JPEG dan
  WebP (90), [[dialog.transparent_background]] (aktif),
  [[dialog.pdf_standard]], [[dialog.output_condition]], dan [[dialog.bleed]].
  [[options.export_remember]] menyimpan pilihan dari ekspor terakhir.
- [[options.import_section]]: [[options.import_place]]
  [[options.import_as_in_file]] (bawaan) atau [[options.import_view_center]].
- [[options.cdr_save_section]]: [[dialog.cdr_version]] yang dipakai untuk
  menyimpan gambar baru sebagai `.cdr`.

## Penjiplakan Bitmap {#bitmap-tracing}

![Halaman Penjiplakan Bitmap pada opsi global](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]] salah satu preset (awalnya Clipart).
[[options.trace_starts_with]] [[dialog.remove_background]] (aktif) dan
[[dialog.delete_original]] (nonaktif). Lihat
[Menjiplak bitmap](../bitmaps/tracing.md).

## Peringatan {#warnings}

![Halaman Peringatan pada opsi global](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]],
[[options.warn_missing_fonts]], dan [[options.warn_shelf_remove]] (semuanya
aktif). Setiap pesan juga memiliki kotak centang untuk menghentikannya;
centang opsinya di sini untuk melihatnya lagi.

[[options.ask_missing_fonts]] (aktif) menampilkan kotak dialog Font yang
Hilang saat gambar dibuka dengan font yang tidak terpasang; lihat
[Font yang hilang](../text/missing-fonts.md). Opsi ini terpisah dari pesan
biasa di atas.

## Lokasi Berkas {#file-locations}

![Halaman Lokasi Berkas pada opsi global](../../img/customize-global-options-file-locations.png)

Menampilkan tempat preferensi dan cadangan otomatis disimpan. Preferensi
berada di folder `tracedraw-settings` di samping program bila folder itu ada
(salinan portabel), selain itu di tempat yang disediakan sistem Anda untuk
pengaturan program; versi peramban menyimpannya di peramban.
