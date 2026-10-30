# Efek bitmap

Efek bitmap mengubah piksel: mengaburkan, mempertajam, menyesuaikan warna,
menambahkan tekstur dan goresan artistik. Efek ini bekerja pada bitmap dan juga
pada objek vektor, yang kemudian digambar sebagai gambar hasil efek sambil
tetap mempertahankan bentuk, isian, dan garis luarnya sendiri.

Efek disimpan terpisah dari piksel asli: Anda dapat menyunting,
menyembunyikan, mengubah urutan, atau menghapus efek mana pun nanti, dan
menghapus efek terakhir mengembalikan gambar aslinya.

## Untuk menerapkan efek bitmap {#apply}

1. Pilih satu objek atau lebih.
2. Pilih kelompok di menu [[menu.effects]] lalu sebuah efek.
3. Efek tanpa pengaturan langsung diterapkan. Efek lainnya membuka kotak
   dialog dengan pratinjau **sebelum** dan **sesudah** dari objek pertama yang
   dipilih: atur nilainya lalu klik [[dialog.ok]]. Pengaturannya berada
   di panel di sebelah kanan; di bawahnya **Atur ulang** mengembalikan setiap
   pengaturan ke bawaannya, dan dua panah mengurungkan dan mengulangi
   perubahan yang dibuat di kotak dialog.

![Kotak dialog efek bitmap](../../img/effects-bitmap-effects.png)

Perubahan pada seluruh pilihan adalah satu langkah urungkan. Bila bitmap
dipilih, <kbd>Ctrl+B</kbd> membuka [[fx.brightness_contrast_intensity]],
<kbd>Ctrl+Shift+B</kbd> [[fx.color_balance]], dan <kbd>Ctrl+Shift+U</kbd>
[[fx.hue_saturation_lightness]].

## Kelompok efek {#groups}

| Kelompok | Efek |
|---|---|
| [[menu.effects.3d]] | [[fx.cylinder]], [[fx.emboss]], [[fx.page_curl]], [[fx.pinch_punch]], [[fx.sphere]] |
| [[menu.effects.adjust]] | [[fx.auto_adjust]], [[fx.image_adjustments]], [[fx.contrast_enhancement]], [[fx.local_equalization]], [[fx.target_balance]], [[fx.tone_curve]], [[fx.brightness_contrast_intensity]], [[fx.color_balance]], [[fx.gamma]], [[fx.hue_saturation_lightness]], [[fx.selective_color]], [[fx.replace_colors]], [[fx.desaturate]], [[fx.channel_mixer]] |
| [[menu.effects.art_strokes]] | [[fx.charcoal]], [[fx.conte_crayon]], [[fx.crayon]], [[fx.cubist]], [[fx.impressionist]], [[fx.palette_knife]], [[fx.pastels]], [[fx.pen_ink]], [[fx.pointillist]], [[fx.scraperboard]], [[fx.sketch_pad]], [[fx.watercolor]], [[fx.felt_marker]], [[fx.paper_grain]] |
| [[menu.effects.blur]] | [[fx.directional_smooth]], [[fx.gaussian_blur]], [[fx.jagged_edges]], [[fx.low_pass]], [[fx.motion_blur]], [[fx.radial_blur]], [[fx.smart_blur]], [[fx.smooth]], [[fx.soften]], [[fx.zoom]] |
| [[menu.effects.camera]] | [[fx.colorize]], [[fx.diffuse]], [[fx.photo_filter]], [[fx.sepia_toning]], [[fx.vintage_photo]] |
| [[menu.effects.color_transform]] | [[fx.bit_planes]], [[fx.halftone]], [[fx.psychedelic]], [[fx.solarize]] |
| [[menu.effects.contour_group]] | [[fx.edge_detect]], [[fx.find_edges]], [[fx.trace_contour]] |
| [[menu.effects.correction]] | [[fx.dust_and_scratch]] |
| [[menu.effects.creative]] | [[fx.crystallize]], [[fx.fabric]], [[fx.frame]], [[fx.glass_tiles]], [[fx.mosaic]], [[fx.scatter]], [[fx.tinted_glass]], [[fx.stained_glass]], [[fx.vignette]], [[fx.vortex]] |
| [[menu.effects.custom]] | [[fx.bump_map]] |
| [[menu.effects.distort]] | [[fx.blocks]], [[fx.displace]], [[fx.mesh_warp]], [[fx.offset]], [[fx.pixelate]], [[fx.ripple]], [[fx.swirl]], [[fx.tile]], [[fx.wet_paint]], [[fx.whirlpool]], [[fx.wind]] |
| [[menu.effects.noise]] | [[fx.add_noise]], [[fx.maximum]], [[fx.median]], [[fx.minimum]], [[fx.remove_moire]], [[fx.remove_noise]] |
| [[menu.effects.sharpen]] | [[fx.adaptive_unsharp]], [[fx.directional_sharpen]], [[fx.high_pass]], [[fx.sharpen]], [[fx.unsharp_mask]] |
| [[menu.effects.texture]] | [[fx.cobblestone]], [[fx.wrinkles]], [[fx.etching]], [[fx.plastic]], [[fx.relief_sculpture]], [[fx.stone]] |
| [[menu.effects.transform]] | [[fx.deinterlace]], [[fx.invert_colors]], [[fx.posterize]] |

## Dialog khusus {#special-dialogs}

- **[[fx.image_adjustments]]** membuka kotak dialog yang lebih besar: pratinjau
  besar yang dapat Anda zum, geser, dan putar, ditampilkan sendiri,
  berdampingan, atau terbelah; **Sesuaikan otomatis** dan pemilih titik putih
  dan hitam; penggeser untuk suhu, rona warna, saturasi, kecerahan, kontras,
  sorotan, bayangan, dan nada tengah, dengan histogram; urungkan dan ulangi di
  dalam dialog; serta cuplikan pengaturan untuk dibandingkan.
- **[[fx.contrast_enhancement]]** menampilkan histogram dengan penanda yang
  dapat diseret untuk tingkat masukan dan keluaran, pipet untuk hitam dan
  putih, serta **Sesuaikan otomatis**.
- **[[fx.tone_curve]]** menyunting kurva per kanal (utama, merah, hijau, biru)
  dalam empat gaya (kurva, lurus, tangan bebas, gamma), di atas histogram;
  kurva dapat disimpan dan dimuat.
- Warna gambar, seperti di [[fx.replace_colors]], memiliki pipet: klik
  pratinjau untuk mengambil warna di bawah penunjuk.

## Menyunting efek sebuah objek {#fx-section}

Bagian [[docker.fx]] di panel [[docker.properties]] mencantumkan efek objek
yang dipilih, yang pertama diterapkan berada di atas. Setiap baris memiliki
kotak centang untuk menampilkan atau menyembunyikan efek, **Sunting** (membuka
kotak dialognya), **Naikkan**, **Turunkan**, dan **Hapus**. **Tambah efek**
membuka kelompok-kelompok efek.

[[menu.effects > menu.effects.flatten]] membuat efek menjadi permanen: bitmap
mempertahankan apa yang ditampilkannya, dan objek vektor menjadi bitmap.

## Pemilih Efek {#effect-picker}

[[menu.effects > menu.effects.picker]] membuka jendela yang tetap terbuka
selama Anda bekerja, dengan setiap efek sebagai gambar kecil. Ketik di kolom
pencarian untuk menemukan efek, atau pilih sebuah kelompok atau **Terbaru**
dari daftar. Bila ada yang dipilih, klik sebuah ubin untuk menerapkan efeknya.
Anda juga dapat menerapkan efek sebagai [lensa](lens.md).

## Efek yang menyebar melewati tepi {#inflate}

Efek buram dan sejenisnya dapat melampaui tepi bitmap. Dengan
[[menu.bitmaps > menu.bitmaps.inflate > menu.bitmaps.inflate_auto]] aktif
(seperti bawaannya), bitmap membesar untuk memberi ruang bagi efek itu. Lihat
[Menyampel ulang dan meluruskan](../bitmaps/resample-and-straighten.md#inflate).

Untuk menyimpan bitmap beserta efeknya sebagai berkas PNG, klik kanan bitmap
lalu pilih [[menu.bitmaps.save_bitmap_as]].
