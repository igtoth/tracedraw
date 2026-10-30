# Menjiplak bitmap

Menjiplak mengubah bitmap menjadi bentuk vektor yang dapat Anda sunting, warnai
ulang, dan skalakan tanpa kehilangan kualitas: logo hasil pindai, sketsa,
foto.

## Jiplak cepat {#quick-trace}

Pilih bitmap lalu pilih [[menu.bitmaps > menu.bitmaps.quick_trace]]. Perintah
ini memakai preset yang dipilih di halaman [[options.bitmap_tracing]] pada
opsi global (awalnya Clipart).

[[menu.bitmaps.repeat_last_trace]] menjiplak ulang bitmap terpilih seperti jiplakan terakhir, tanpa kotak dialog, dan [[menu.bitmaps.last_trace_settings]] membuka kotak dialog dengan pengaturan jiplakan terakhir. Menu klik kanan bitmap juga memiliki perintah ini, bersama [[menu.bitmaps.centerline_trace]] dan [[menu.bitmaps.outline_trace]].

## Kotak dialog Jiplak Bitmap {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] dan
[[menu.bitmaps.centerline_trace]] berisi daftar preset; pilih salah satu untuk
membuka kotak dialog jiplak.

- **Jiplak garis luar** mengisi area warna dengan bentuk:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]], dan [[menu.bitmaps.trace_high_quality]].
- **Jiplak garis tengah** mengikuti garis dengan kurva terbuka:
  [[menu.bitmaps.trace_technical]] dan [[menu.bitmaps.trace_line_drawing]].

![Kotak dialog Jiplak Bitmap](../../img/bitmaps-tracing.png)

Kotak dialog ini adalah jendela yang dapat Anda ubah ukurannya dan
maksimalkan. Pratinjau mengisi sisi kirinya, pengaturan berada di panel di
sebelah kanan, dan jiplakan dibuat ulang setiap kali Anda mengubah pengaturan:
tidak ada yang perlu ditekan untuk melihat hasilnya. Di bawah pratinjau,
sebuah baris menghitung **kurva**, **simpul**, dan **warna** jiplakan. Klik
[[dialog.ok]] untuk meletakkan bentuk-bentuk di atas bitmap, yang paling gelap
lebih dahulu, atau [[dialog.cancel]] untuk membiarkan bitmap apa adanya.

## Pratinjau {#preview}

Daftar [[dialog.trace_preview_label]] memilih apa yang ditampilkan pratinjau:

- [[dialog.trace_view_before_after]]: bitmap di atas dan jiplakannya di bawah,
  atau [[dialog.view_side_by_side]]; [[dialog.before]] menampilkan bitmap
  saja.
- [[dialog.trace_view_large]]: jiplakan saja, sebesar pratinjau.
- [[dialog.trace_view_wireframe]]: kurva-kurva jiplakan digambar sebagai garis
  tipis di atas bitmap. Penggeser [[dialog.trace_overlay]] muncul di samping
  daftar: ia mengatur seberapa kuat bitmap tampak di bawah garis, dari 0
  (garis saja) sampai 100 (bitmap apa adanya), awalnya 80.

Tombol-tombol setelah daftar memperbesar agar pas, memperkecil, memperbesar,
dan menggeser. Roda tetikus memperbesar di sekitar penunjuk dan seretan
memindahkan gambar; kedua panel bergerak bersama. Bagian yang transparan
menampilkan papan catur.

## Pengaturan {#settings}

Tab [[dialog.trace_tab_settings]] berisi:

- [[dialog.trace_type]] Garis luar atau Garis tengah, dan
  [[dialog.trace_image_type]] preset. Memilih salah satu mengatur penggeser di
  bawahnya ke nilai yang cocok untuk jenis gambar itu.
- [[dialog.detail]] (0 sampai 100): detail lebih banyak mempertahankan area
  yang lebih kecil dan garis yang lebih halus; detail lebih sedikit
  menghaluskan bintik dan butiran.
- [[dialog.smoothing]] (0 sampai 100, awalnya 25): kurva lebih halus dengan
  simpul lebih sedikit.
- [[dialog.corner_smoothness]] (0 sampai 100): sudut lebih bulat.
- [[dialog.trace_remove]]: apa yang dikeluarkan dari jiplakan. **Warna latar
  belakang** menghapus latar belakang di tempat ia mencapai tepi gambar,
  **Seluruh warna dari gambar** menghapus setiap area berwarna itu, **Tidak
  ada** menjiplak semuanya. [[dialog.trace_remove_auto]] menemukan warna latar
  belakang di sepanjang tepi gambar; [[dialog.trace_remove_specify]] memakai
  warna contoh, yang dapat Anda ubah atau ambil dari pratinjau dengan pipet
  di sampingnya.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (awalnya aktif)
  membuat satu kurva dari setiap area sewarna; jika nonaktif, setiap bidang
  terpisah menjadi kurva sendiri. [[dialog.trace_remove_overlap]] menjaga
  setiap bentuk pada pikselnya sendiri; jika nonaktif, bentuk-bentuk sedikit
  masuk ke bawah bentuk di atasnya, sehingga tidak ada celah di antara
  tetangga. [[dialog.trace_group_by_color]] menaruh kurva-kurva setiap warna
  dalam grup sendiri; tersedia bila area sewarna belum digabungkan.
- [[dialog.delete_original]]: bitmap hilang saat jiplakan diletakkan.

Pengaturan yang tidak berlaku untuk jenis jiplakan saat ini ditampilkan
abu-abu: jiplakan garis tengah hanya membaca detail dan penghalusan.

## Warna {#colors}

Tab [[dialog.trace_tab_colors]] mengatur [[dialog.color_mode]] (RGB, skala
abu-abu, atau hitam dan putih) dan [[dialog.trace_number_of_colors]] (2 sampai
256), dan mendaftar warna-warna jiplakan dengan jumlah kurva masing-masing.
Centang dua warna atau lebih dan klik [[dialog.trace_merge_colors]] untuk
menggabungkannya ke warna pertama; klik contoh suatu warna untuk menggambar
kurvanya dengan warna lain. Mengubah pengaturan yang menjiplak gambar lagi
melupakan suntingan ini, yang milik jiplakan sebelumnya.

## Setel ulang, urungkan, dan ulangi {#steps}

Di bawah panel pengaturan, [[dialog.reset]] mengembalikan pengaturan preset,
dan dua panah melangkah mundur dan maju melalui setiap perubahan yang dibuat
di kotak dialog: setiap pelepasan penggeser, pilihan daftar, kotak centang,
suntingan warna, dan Setel ulang adalah satu langkah. Perubahan yang dibuat
setelah mengurungkan membuang langkah-langkah yang diurungkan.

[[dialog.trace_options]] membuka halaman [[options.bitmap_tracing]] pada opsi
global, tempat Anda memilih preset jiplak cepat dan pilihan awal setiap
jiplakan, lalu kembali ke kotak dialog. Tombol [[dialog.help_button]] membuka
halaman ini.
