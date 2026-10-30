# Isian

Isian mewarnai bagian dalam objek tertutup. TraceDraw memiliki enam jenis
isian: [[docker.fill_none]], [[docker.fill_uniform]] (satu warna rata),
[[docker.fill_fountain]] (warna-warna yang berbaur satu sama lain),
[[docker.fill_pattern]], [[docker.fill_texture]], dan [[docker.fill_mesh]]
(warna yang mengalir di antara simpul-simpul kisi).

## Cara mengisi objek {#ways}

- Klik warna di palet untuk memberi pilihan isian seragam.
- Gunakan bagian [[docker.fill]] di panel [[docker.properties]]
  (<kbd>F11</kbd>): pilih jenis isian dengan deretan gambar, lalu
  pengaturannya.
- Gunakan alat [[tool.interactive_fill]] (<kbd>G</kbd>) untuk mengatur gradien
  langsung pada objek.
- Gunakan alat [[tool.area_fill]] untuk mengisi area yang dikelilingi beberapa
  objek.
- Gunakan alat [[tool.mesh_fill]] (<kbd>M</kbd>) untuk isian jala.

Bila tidak ada yang dipilih, isian yang Anda pilih menjadi isian objek baru.

## Isian seragam {#uniform}

Isian seragam adalah satu warna, dalam model warna apa pun. Pilih dari palet,
panel [[docker.color]] (lihat [Panel Warna](color-docker.md)), atau bagian
isian di panel [[docker.properties]].

## Isian gradien {#gradient}

Gradien memadukan dua warna atau lebih. Pengaturannya, di bagian isian panel
[[docker.properties]]:

- Jenisnya: [[fill.linear]] (sepanjang garis), [[fill.radial]] (dari pusat ke
  luar), [[fill.conical]] (mengelilingi pusat), [[fill.square]] (dalam persegi
  yang makin besar), atau [[fill.shaped]] (mengikuti garis luar objek, dari
  tepi ke dalam).
- [[docker.gradient_presets]]: gradien siap pakai sebagai titik awal.
- **Titik warna**: warna-warna dan letaknya di sepanjang gradien, 2 sampai 64.
  Pilih titik untuk mengubah warna atau posisinya; [[docker.add_stop]] dan
  [[docker.remove_stop]] menambah dan menghapus titik.
  [[docker.import_gradient]] mengambil warna dari berkas gradien `.ggr`.
- **Sudut** gradien linear, kerucut, dan persegi, [[docker.edge_pad]]
  (seberapa banyak tiap ujung berwarna padat), dan geseran pusat gradien
  radial, kerucut, dan persegi.
- [[docker.repeat]]: [[docker.repeat_none]], [[docker.repeat_sawtooth]]
  (warna berulang lagi dari awal), atau [[docker.repeat_mirror]] (warna
  bergerak bolak-balik), dengan jumlah [[docker.repeat_count]].
- [[docker.bilinear]]: warna bergerak dari yang pertama ke yang terakhir lalu
  kembali.
- [[docker.invert_colours]]: warna bergerak dalam urutan terbalik.

Sebuah jalur di bawah jenis menampilkan seluruh urutan warna seperti yang akan
digambar.

## Kotak dialog Edit Isian {#edit-fill}

<kbd>F11</kbd> membuka kotak dialog [[edit_fill.title]] pada isian pilihan.
Untuk gradien, kotak ini menampilkan pratinjau, [[edit_fill.fill_picker]],
jenis, pengulangan, dan pita warna: klik node untuk memilihnya, seret untuk
memindahkannya, klik ganda untuk menambah atau menghapus node. Di bawah pita:
warna, transparansi, posisi, dan [[edit_fill.midpoint]] node, serta
[[edit_fill.reverse]]. [[edit_fill.transform]] berisi rotasi, pusat, dan
bantalan tepi. **OK** menerapkan yang Anda ubah, dengan
[[edit_fill.overprint]]. Selama kotak dialog terbuka, seleksi menampilkan isian saat Anda mengubahnya; **Batal** atau <kbd>Esc</kbd> mengembalikan isian lama.

![Kotak dialog Edit Isian](../../img/color-fills-edit-fill.png)

## Alat Isian Interaktif {#interactive-fill}

Alat [[tool.interactive_fill]] (<kbd>G</kbd>) menyunting isian langsung pada
objek.

1. Pilih alat lalu seret melintasi objek: Anda mendapat gradien linear dari
   tempat Anda menekan hingga tempat Anda melepas. Tahan <kbd>Ctrl</kbd> untuk
   membatasi sudut ke kelipatan 15 derajat. Satu klik memberi isian bawaan.
2. Seret simpul pada objek untuk mengubah gradien: dua simpul persegi pada
   gradien linear mengubah arah dan panjangnya; simpul gradien radial,
   kerucut, atau persegi memindahkan pusatnya.
3. Seret warna palet ke sebuah simpul untuk mewarnainya ulang, atau ke garis di
   antara simpul untuk menambah warna di sana.

Bilah properti berisi jenis isian dan pengaturannya: warna isian seragam;
untuk gradien, jenisnya, warna awal dan akhir, sudut, dan bantalan tepi.

## Alat Isi Area {#area-fill}

Alat [[tool.area_fill]] mengisi area tertutup apa pun, bahkan yang terbentuk
dari beberapa objek yang saling tumpang tindih, dengan membuat objek baru dari
area itu. Objek di sekitarnya tetap seperti semula.

1. Pilih alat [[tool.area_fill]].
2. Pilih isian dan garis luar di bilah properti: **Tentukan** (warnanya, dan
   untuk garis luar juga lebarnya, di sampingnya), **Gunakan bawaan** (isian
   dan garis luar objek baru; sebuah warna bila bawaannya tanpa isian, agar
   area terlihat), atau **Tidak ada**.
3. Klik di dalam area.

Objek baru diletakkan di atas lapisan aktif dan dipilih. Klik pada garis luar,
atau di area yang tidak dikelilingi apa pun, tidak menghasilkan apa-apa.

## Isian jala {#mesh-fill}

Isian jala adalah kisi simpul di atas objek, setiap simpul dengan warna dan
transparansinya sendiri; warna mengalir halus di antara simpul. Cocok untuk
bayangan yang realistis.

1. Pilih alat [[tool.mesh_fill]] (<kbd>M</kbd>) lalu atur baris dan kolom di
   bilah properti (awalnya 2 kali 2, hingga 50).
2. Klik sebuah objek. Objek mendapat jala dengan warna isiannya.
3. Klik simpul untuk memilihnya, lalu klik warna di palet untuk mewarnainya.
   Seret simpul untuk membelokkan aliran warna.
4. Klik ganda di dalam jala untuk menambah baris atau kolom melalui simpul
   terdekat; pilih simpul lalu tekan <kbd>Delete</kbd> untuk menghapus baris
   dan kolomnya.

Bilah properti juga berisi **Warna halus**, yang memadukan warna dengan lebih
lembut, **transparansi** simpul yang dipilih, dan **Hapus jala**, yang
menghapus isian jala. Mengubah baris dan kolom jala tetap mempertahankan
warnanya.

## Isian pola dan tekstur {#pattern-texture}

- Isian [[docker.fill_pattern]] mengulang sebuah ubin: dua warna, pola vektor,
  atau bitmap. Lihat [Isian pola](patterns.md).
- Isian [[docker.fill_texture]] adalah tekstur hasil olahan yang tampak alami:
  [[texture.clouds]], [[texture.marble]], [[texture.noise]], atau
  [[texture.wood]], dengan warna dan skalanya.

## Beberapa isian pada satu objek {#several-fills}

Di bagian isian panel [[docker.properties]], [[docker.add_fill]] melukis isian
lain di atas isian objek itu sendiri, dimulai sebagai salinan untuk diubah.
Isian berikutnya tampil sebagai "Isian 2", "Isian 3", dan seterusnya, masing-
masing dengan penyuntingnya sendiri dan tombol untuk menghapusnya. Bagian garis
luar dapat menambahkan garis luar lain dengan cara yang sama.

Isian juga dapat dilukis pada garis luar; lihat
[Mengatur garis luar](../lines-shapes/outlines.md#outline-pen).
