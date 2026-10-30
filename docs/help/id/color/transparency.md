# Transparansi

Transparansi membuat apa yang ada di bawah objek terlihat menembusnya.
Transparansi dapat sama di seluruh objek, berubah di sepanjang objek seperti
gradien, atau mengikuti pola atau tekstur.

## Menerapkan transparansi {#apply}

- Dengan alat [[tool.transparency]]: klik objek, lalu pilih jenis
  transparansi di bilah properti, atau seret melintasi objek untuk
  transparansi gradien dari buram (tempat Anda menekan) hingga transparan
  (tempat Anda melepas). Simpul gradien lalu tampil pada objek, seperti pada alat [[tool.interactive_fill]]: seret awal, akhir, atau simpul miring yang bulat; seret batang di antara dua simpul (titik tengah) untuk memindahkan tempat campuran mencapai setengah; klik ganda garis untuk menambah simpul, dan simpul untuk menghapusnya.
- Di bagian [[docker.transparency]] pada panel [[docker.properties]].

## Pengaturan {#settings}

- Jenisnya: [[transparency.none]], [[transparency.uniform]] (bawaan),
  [[transparency.fountain]] (linear, radial, kerucut, persegi, atau berbentuk,
  seperti isian gradien), [[transparency.pattern]], atau
  [[transparency.texture]].
- Jumlahnya: 0 sampai 100% (awalnya 50%). Untuk gradien, titik warnanya
  berupa gradasi abu-abu: putih berarti buram, hitam berarti sepenuhnya
  transparan.
- [[transparency.merge_mode]]: cara warna bercampur dengan apa yang ada di
  bawahnya. Normal (bawaan) hanya memperlihatkan yang di bawah; mode lain,
  seperti Tambah, Kurangi, Selisih, Kalikan, Bagi, Bila lebih terang, Bila
  lebih gelap, Rona, Saturasi, Warna, dan Balikkan, menggabungkan warna dengan
  cara lain.
- Apakah berlaku untuk [[transparency.target_fill]],
  [[transparency.target_outline]], atau [[transparency.target_all]].
- **Bekukan**: mempertahankan apa yang terlihat menembus seperti saat ini,
  meskipun objek di bawahnya berpindah.

Untuk menghapusnya, klik [[toolbar.clear_transparency]] di ujung bilah properti (atau pilih [[transparency.none]] sebagai jenisnya).

Untuk menyalin transparansi objek lain, gunakan
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
Untuk area tembus pandang yang ditentukan oleh kecerahan objek, lihat
[Masker opasitas](../objects/clip-frames.md#opacity-mask).
