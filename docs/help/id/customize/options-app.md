# Opsi TraceDraw

[[menu.tools > menu.tools.options > menu.tools.options_app]] (<kbd>Ctrl+J</kbd>)
mengatur cara kerja program. Pilih halaman dari daftar di sebelah kiri.

## Umum {#general}

![Halaman Umum pada opsi TraceDraw](../../img/customize-options-app-general.png)

- [[options.on_startup]]: tampilkan Layar Sambutan, mulai dokumen baru, atau
  buka dokumen yang terakhir disunting.
- [[options.show_new_document_dialog]] (aktif): menanyakan pengaturan gambar
  baru; lihat [Membuat gambar](../get-started/new-document.md).
- [[options.show_splash]] (aktif).
- [[options.hardware_acceleration]] (aktif): jendela digambar dengan kartu grafis yang dipilih di sebelahnya ([[options.gpu_automatic]] pada awalnya); nonaktif, dengan perender perangkat lunak komputer. Berlaku pada mulai berikutnya. Jika jendela tidak bisa terbuka dengan cara itu, TraceDraw mulai lagi dengan perender perangkat lunak, lalu dengan OpenGL, dan mengingat yang berhasil.
- [[options.undo_levels]]: berapa langkah yang dapat Anda urungkan (awalnya
  150).

## Tampilan {#display}

![Halaman Tampilan pada opsi TraceDraw](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: tampilkan warna seperti saat dicetak.
- [[options.show_tooltips]] (aktif), [[options.hide_bbox_curve_tools]], dan
  [[options.node_tracking]] (nonaktif: bila aktif, alat [[tool.pick]] dapat
  menyeret simpul kurva yang dipilih).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (bawaan) atau
  [[options.wheel_scroll]].
- [[options.ruler_calibration]]: tempelkan penggaris ke layar dan ubah
  [[options.screen_resolution]] sampai tandanya cocok, agar zum 100%
  menampilkan ukuran sebenarnya. [[options.reset]] mengembalikan nilai 96.

## Sunting {#edit}

![Halaman Sunting pada opsi TraceDraw](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: kelipatan sudut <kbd>Ctrl</kbd> saat memutar
  dan menggambar (awalnya 15 derajat).
- [[options.drawing_precision]]: jumlah angka desimal di bilah properti dan
  bilah status (awalnya 3).
- [[options.focus_inactive]] (awalnya 20%) dan
  [[options.focus_double_click]]; lihat [Mode fokus](../objects/focus-mode.md).

## Simpul dan Gagang {#nodes-and-handles}

![Halaman Simpul dan Gagang pada opsi TraceDraw](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: kecil, sedang, atau besar.
- [[options.node_shape]] untuk simpul [[options.node_cusp]],
  [[options.node_smooth]], dan [[options.node_symmetrical]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: [[options.main_color]] simpul dan gagang yang dipilih
  serta [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Bingkai Potong {#clip-frame}

![Halaman Bingkai Potong pada opsi TraceDraw](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (bawaan),
  [[options.auto_center_always]], atau [[options.auto_center_never]].
- [[options.empty_frame_lines]] (aktif).

Lihat [Menempatkan objek di dalam bingkai](../objects/clip-frames.md).

## Penempelan {#snapping}

![Halaman Penempelan pada opsi TraceDraw](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], kisi garis dasar,
  dan [[options.snap_to_pixels]].
- [[options.snapping_radius]] (awalnya 10 piksel).
- [[options.show_snap_marks]] dan [[options.snap_screen_tip]].
- [[options.snap_modes]]: titik-titik tempat objek menempel, dengan
  [[options.select_all]] dan [[options.deselect_all]].

Lihat [Penempelan](../pages-layout/snapping.md).

## Simpan {#save}

![Halaman Simpan pada opsi TraceDraw](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (aktif) menyimpan berkas
  sebelumnya sebagai `backup_of_<name>`, [[options.backup_to]]
  [[options.same_folder]] atau tempat yang dipilih.
- [[options.auto_backup]] (aktif): [[options.backup_every]] 20
  [[options.minutes]], ke folder sementara atau tempat yang dipilih. Setelah
  program mogok, cadangan akan ditawarkan; lihat
  [Cadangan otomatis dan pemulihan](../get-started/open-and-save.md#recovery).

## Teks {#text}

![Halaman Teks pada opsi TraceDraw](../../img/customize-options-app-text.png)

Halaman ini punya lima tab.

- [[options.display]]: [[options.keyboard_text_increment]] (seberapa banyak
  <kbd>Ctrl+8</kbd> dan <kbd>Ctrl+2</kbd> memperbesar dan memperkecil teks yang
  dipilih, 1 poin), [[options.default_font]] dan [[options.default_size]] untuk
  teks baru, serta [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] dan [[options.hyphenation_patterns]]
  (bawaan, atau kamus pemenggalan untuk bahasa Anda).
- [[docker.fonts]]: [[options.show_recent_fonts]] di atas daftar font,
  [[options.recent_font_count]] (5, hingga 20) dan
  [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  dan [[options.add_to_autocorrect]], agar kata yang Anda koreksi di pemeriksa
  ejaan dikoreksi saat mengetik berikutnya; serta [[options.dictionaries]] yang
  dipakai, lihat [Kamus](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: apa yang dikoreksi saat mengetik; lihat
  [Koreksi otomatis](../text/writing-tools.md#autocorrect).
