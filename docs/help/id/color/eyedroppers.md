# Menyalin warna dan atribut dengan pipet

Alat pipet mengambil warna, atau seluruh kumpulan atribut, dari satu objek dan
memberikannya ke objek lain. Keduanya bekerja dalam dua langkah, yang tampil di
bilah properti: **pilih** (ambil sampel) dan **terapkan** (lukis dengannya).
Setelah mengambil sampel, alat beralih ke terapkan; klik tombol pilih untuk
mengambil sampel lagi. <kbd>Esc</kbd> membuang sampel dan kembali ke alat
[[tool.pick]].

Penunjuk berupa pipet saat alat mengambil sampel dan ember cat saat
menerapkan. Gambar memberikan warnanya meski di dalam grup atau bingkai.

## Pipet Warna {#color-eyedropper}

1. Pilih alat [[tool.color_eyedropper]].
2. Klik objek untuk mengambil warnanya: warna isian dari isian seragam, warna
   pertama dari gradien, atau warna garis luar. Pada bitmap, alat ini mengambil
   warna piksel di bawah penunjuk, dirata-ratakan sesuai ukuran sampel yang
   diatur di bilah properti (1 kali 1, 2 kali 2, atau 5 kali 5 piksel).
   <kbd>Shift</kbd>+klik mengambil warna garis luar lebih dahulu.
3. Klik objek untuk mengisinya dengan warna itu; <kbd>Shift</kbd>+klik untuk
   mewarnai garis luarnya (objek tanpa garis luar mendapat garis tipis). Klik
   di dalam bentuk tertutup yang kosong akan mengisinya.

Bilah properti menampilkan warna sampel.

Saat Anda menggerakkan penunjuk untuk mengambil warna, kotak kecil di
sampingnya menampilkan warna di bawahnya: contoh warna, nama warna bila itu
warna palet atau tinta spot, nilainya dalam mode warna gambar (CMYK atau RGB),
dan nilai heksnya. Pipet pemilih warna menampilkan kotak yang sama.

## Pipet Atribut {#attributes-eyedropper}

Alat [[tool.attributes_eyedropper]] menyalin beberapa properti sekaligus dari
satu objek ke objek lain.

1. Pilih alat [[tool.attributes_eyedropper]].
2. Di bilah properti, pilih apa yang disalin:
   - **Properti**: garis luar, isian, dan teks (fon, ukuran, dan gaya; teksnya
     sendiri tetap). Semua awalnya aktif.
   - **Transformasi**: ukuran, rotasi, dan posisi. Awalnya nonaktif.
   - **Efek**: transparansi, bayangan jatuh, dan efek langsung. Awalnya
     nonaktif.
3. Klik objek sumber.
4. Klik setiap objek tujuan. <kbd>Shift</kbd>+klik justru mengambil sampel
   dari objek lain.

Setiap klik adalah satu langkah urungkan.
