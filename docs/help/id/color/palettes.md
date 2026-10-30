# Palet warna

Palet warna di sepanjang bagian bawah jendela adalah cara tercepat untuk
mewarnai objek. Di bawahnya, **palet dokumen** mengumpulkan warna yang dipakai
gambar Anda. Anda dapat membuka palet lain, membuat palet sendiri, dan
menyuntingnya.

## Memakai palet {#using}

| Tindakan | Hasil |
|---|---|
| Klik warna | mengisi pilihan (bila tidak ada yang dipilih, mengatur isian objek baru) |
| Klik kanan warna | mewarnai garis luar pilihan |
| <kbd>Ctrl</kbd>+klik warna | mencampurkan sepersepuluh warna itu ke isian pilihan |
| Klik dan tahan warna | menampilkan gradasi lebih terang, lebih gelap, lebih kusam, dan lebih cerah untuk dipilih |
| Seret warna ke objek | mengisi objek, atau mewarnai garis luarnya bila dilepas di tepinya |
| Klik kotak bercoret | menghapus isian (klik kanan: garis luar) |

Panah di kedua ujung menggulir palet, dan tombol di ujung kanan menampilkan
semua warna sekaligus. Arahkan penunjuk ke sebuah warna untuk melihat nama dan
nilainya. Tombol menu palet di ujung kiri membuka menu palet.

Palet dapat dipindahkan ke tepi jendela yang lain, atau dibuat mengambang:
seret dengan pegangan bertitiknya.

Di setiap sisi palet bisa berada di tepi jendela atau di samping gambar, di antara gambar dan panel atau kotak alat di sisi itu. Saat Anda menyeretnya, pita biru menunjukkan tempatnya nanti; klik kanan pegangannya untuk memilih tempat dari daftar.

## Palet dokumen {#document-palette}

Setiap warna yang Anda terapkan ke objek masuk ke palet dokumen, yang disimpan
bersama gambar. Pipetnya menambahkan warna di bawah penunjuk, dan menunya
berisi **Perbarui otomatis** (untuk berhenti menambahkan warna dengan
sendirinya), **Tambah dari pilihan**, **Tambah dari dokumen**, **Hapus warna**,
dan **Atur ulang palet** (hanya mempertahankan warna yang masih dipakai
gambar). Anda juga dapat menyeret warna atau objek ke palet ini. Untuk
menampilkan atau menyembunyikannya, lihat
[Opsi kustomisasi](../customize/customization.md#color-palette).

## Panel Palet {#palettes-docker}

Panel [[docker.palettes]] mencantumkan palet yang Anda miliki, dengan jumlah
warna di masing-masing. Buka dengan
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]].

![Panel Palet](../../img/color-palettes.png)

- Centang palet untuk menampilkannya di bawah jendela gambar.
- [[docker.edit]] membukanya di penyunting palet.
- [[menu.window.palette_open]] membuka berkas palet (`.tdpal`, `.gpl`,
  `.ase`, atau `.aco`).
- [[menu.window.palette_from_document]] dan
  [[menu.window.palette_from_selection]] membuat palet dari warna yang dipakai
  di gambar atau di pilihan.
- Warna palet dokumen tampil di bagian bawah; klik salah satunya untuk mengisi
  pilihan, klik kanan untuk mewarnai garis luar.

## Menyunting palet {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]]
membuka penyunting palet.

![Penyunting Palet](../../img/color-palettes-palette-editor.png)

1. Pilih [[dialog.palette]] yang akan disunting, atau klik
   [[dialog.new_palette]] untuk memulai yang baru.
2. Ketik [[dialog.name]].
3. Klik warna untuk memilihnya; [[dialog.add_color]] menambahkan warna saat
   ini dan [[dialog.remove_color]] menghapus warna yang dipilih.
4. Klik [[dialog.save_palette]] untuk menyimpannya sebagai berkas.
5. Klik [[dialog.close]].

Palet bawaan tidak dapat disunting; buat palet baru, atau sunting salinannya.
