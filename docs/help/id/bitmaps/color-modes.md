# Mengubah mode warna bitmap

> *Terjemahan mesin: halaman ini mungkin tertinggal dari versi bahasa Inggris.*

[[menu.bitmaps > menu.bitmaps.mode]] mengonversi bitmap yang dipilih ke mode
warna lain. Setiap bitmap yang dipilih berubah dalam satu langkah urungkan, dan
transparansi tetap dipertahankan. Mode yang memiliki pengaturan membuka kotak
dialog dengan pratinjau sebelum dan sesudah.

| Mode | Hasil |
|---|---|
| [[menu.bitmaps.mode_bw]] | hanya hitam dan putih (lihat di bawah) |
| [[menu.bitmaps.mode_grayscale]] | gradasi abu-abu |
| [[menu.bitmaps.mode_duotone]] | satu sampai empat tinta (lihat di bawah) |
| [[menu.bitmaps.mode_paletted]] | paling banyak 256 warna (lihat di bawah) |
| [[menu.bitmaps.mode_rgb]] | mode bitmap menjadi RGB; piksel tetap apa adanya |
| [[menu.bitmaps.mode_lab]] | warna Lab |
| [[menu.bitmaps.mode_cmyk]] | mode bitmap menjadi CMYK dan pikselnya menampilkan warna yang dapat dicetak tinta, dihitung dengan profil manajemen warna |

Menu mencentang [[menu.bitmaps.mode_rgb]] atau [[menu.bitmaps.mode_cmyk]] bila semua bitmap terpilih berada dalam mode itu, dan bilah status menyebut mode bitmap terpilih, seperti "Bitmap 600 x 400 px (CMYK)". Gambar CMYK dari berkas `.cdr` terbuka dalam mode CMYK, dan berkas `.tdraw` menyimpan modenya. [[menu.tools > menu.tools.convert_colors]] mengatur mode setiap bitmap yang dikonversinya dengan cara yang sama (lihat [Mengonversi warna](../color/convert-colors.md)).

## Hitam dan putih {#black-and-white}

![Kotak dialog Hitam dan Putih](../../img/bitmaps-color-modes-black-and-white.png)

Pilih metode konversi:

- **Seni garis**: putih bila tingkat abu-abu sama dengan atau di atas
  **ambang** (awalnya 128), hitam di tempat lain;
- **Berurutan**: pola titik yang teratur;
- **Halftone** (bawaan): raster titik bulat, garis, atau persegi pada
  **sudut** tertentu (45 derajat) dan jumlah **garis per inci** (60);
- **Distribusi kardinalitas**: tekstur yang tampak acak;
- **Jarvis**, **Stucki**, dan **Floyd-Steinberg**: difusi galat, hasil paling
  halus untuk foto.

**Intensitas** (0 sampai 100, awalnya 50) membuat hasil lebih terang atau lebih
gelap.

## Duotone {#duotone}

![Kotak dialog Duotone](../../img/bitmaps-color-modes-duotone.png)

Duotone mencetak gambar skala abu-abu dengan beberapa tinta, sehingga
menghasilkan nada yang kaya dengan sedikit warna.

1. Pilih jenisnya: Monotone, Duotone (bawaan), Tritone, atau Quadtone, untuk
   satu sampai empat tinta.
2. Klik sebuah tinta untuk mengubah warnanya.
3. Sunting kurvanya: sumbu mendatar adalah tingkat abu-abu (hitam di kiri),
   sumbu tegak adalah seberapa banyak tinta itu dicetak. Seret di ruang kosong
   untuk menambah titik, seret titik untuk memindahkannya, klik ganda titik
   atau seret keluar dari kisi untuk menghapusnya. **Tampilkan semua** juga
   menggambar kurva tinta lainnya.
4. **Simpan** dan **Muat** menyimpan tinta dan kurva dalam sebuah berkas.

## Berpalet {#paletted}

![Kotak dialog Berpalet](../../img/bitmaps-color-modes-paletted.png)

- **Palet**: Warna Seragam, VGA Standar (16 warna), Adaptif dan Teroptimasi
  (warna gambar itu sendiri, dikurangi hingga jumlah **warna** yang Anda
  atur, sampai 256), Skala Abu-abu, Sistem, atau Kustom (palet dokumen).
- **Dithering**: Tidak ada, Berurutan, Jarvis, Stucki, atau Floyd-Steinberg,
  dengan intensitas dither.

Palet yang dihasilkan tampil di bawah pengaturan.

## Mewarnai bitmap hitam putih {#monochrome}

Bila hanya bitmap dua warna yang dipilih, klik warna palet untuk mewarnai
latar belakangnya, dan klik kanan untuk mewarnai latar depannya.
