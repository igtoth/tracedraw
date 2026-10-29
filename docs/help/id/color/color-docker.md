# Panel Warna

Panel [[docker.color]] (<kbd>Shift+F11</kbd>) mencampur warna dan
menerapkannya ke isian atau garis luar pilihan. Alat [[tool.outline_color]]
(<kbd>Shift+F12</kbd>) juga membukanya. Buka dari
[[menu.window > menu.window.dockers]].

![Panel Warna](../../img/color-color-docker.png)

## Memilih warna {#choose}

1. Pilih objek. Panel dimulai dari warna isian objek pertama.
2. Campur warna baru di salah satu tampilan:
   - [[docker.color_viewers]]: klik atau seret di dalam persegi untuk mengatur
     saturasi (kiri ke kanan) dan kecerahan (bawah ke atas), dan di jalur di
     sampingnya untuk mengatur rona;
   - [[docker.color_sliders]]: satu penggeser per komponen dari
     [[docker.model]]: RGB, CMYK (bawaan), HSB, HSL, Lab, Skala Abu-abu, atau
     YIQ;
   - [[docker.color_palettes]]: contoh warna dari sebuah palet.

   Tiga tombol di bagian atas menampilkan atau menyembunyikan setiap tampilan;
   kombinasi apa pun dapat ditampilkan. Anda juga dapat mengetik nilai
   `#rrggbb`.
3. Klik [[docker.fill]] atau [[docker.outline]] di bagian bawah panel untuk
   menerapkan warna baru.
   Kunci ([[docker.lock_colors]]) mengikat isian dan garis luar, sehingga salah
   satu tombol menerapkan warna pada keduanya. Tombol "..."
   ([[docker.more_actions]]) membuka tindakan lain: [[docker.no_fill]],
   [[docker.no_outline_action]], [[docker.swap_colors]],
   [[docker.add_to_document_palette]], [[docker.add_to_spot_colors]], dan
   [[docker.add_to_color_styles]].

Warna lama dan warna baru tampil berdampingan. Bila warna baru tidak dapat
dicetak tepat dengan tinta CMYK, panel menampilkan peringatan.

## Mengambil sampel warna {#sample}

Klik [[docker.eyedropper]] lalu klik di mana saja pada gambar untuk mengambil
warna di sana. Lalu klik [[docker.use_sampled]] untuk menjadikannya warna
baru. Lihat juga [Pipet](eyedroppers.md).

## Tinta spot {#spot-inks}

Di bawah tombol-tombol, [[docker.spot_inks]] mencantumkan tinta spot dalam
gambar; lihat [Warna spot](spot-colors.md).
