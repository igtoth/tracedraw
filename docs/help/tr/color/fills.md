# Dolgular

Dolgu, kapalı bir nesnenin içini boyar. TraceDraw'da altı tür dolgu vardır:
[[docker.fill_none]], [[docker.fill_uniform]] (tek bir düz renk),
[[docker.fill_fountain]] (birbirine karışan renkler),
[[docker.fill_pattern]], [[docker.fill_texture]] ve [[docker.fill_mesh]]
(bir ızgaranın düğümleri arasında akan renkler).

## Bir nesneyi doldurmanın yolları {#ways}

- Seçime düz bir dolgu vermek için paletteki bir renge tıklayın.
- [[docker.properties]] panelinin (<kbd>F11</kbd>) [[docker.fill]] bölümünü
  kullanın: resim sırasıyla dolgu türünü, ardından ayarlarını seçin.
- Degradeleri doğrudan nesnenin üzerinde ayarlamak için
  [[tool.interactive_fill]] aracını (<kbd>G</kbd>) kullanın.
- Birkaç nesnenin çevrelediği bir alanı doldurmak için [[tool.area_fill]]
  aracını kullanın.
- Ağ dolgusu için [[tool.mesh_fill]] aracını (<kbd>M</kbd>) kullanın.

Hiçbir şey seçili değilken seçtiğiniz dolgu yeni nesnelerin dolgusu olur.

## Düz dolgular {#uniform}

Düz dolgu, herhangi bir renk modelinde tek bir renktir. Onu paletten,
[[docker.color]] panelinden (bkz. [Renk paneli](color-docker.md)) veya
[[docker.properties]] panelinin dolgu bölümünden seçin.

## Degrade dolgular {#gradient}

Degrade iki veya daha fazla rengi karıştırır. [[docker.properties]]
panelinin dolgu bölümündeki ayarları:

- Tür: [[fill.linear]] (bir çizgi boyunca), [[fill.radial]] (merkezden
  dışarıya), [[fill.conical]] (merkezin çevresinde), [[fill.square]]
  (büyüyen karelerle) veya [[fill.shaped]] (nesnenin anahattını izleyerek,
  kenardan içeriye).
- [[docker.gradient_presets]]: başlangıç için hazır degradeler.
- **Duraklar**: renkler ve degrade boyunca nerede durdukları, 2 ile 64 arası.
  Rengini veya konumunu değiştirmek için bir durak seçin;
  [[docker.add_stop]] ve [[docker.remove_stop]] durak ekler ve kaldırır.
  [[docker.import_gradient]] renkleri bir `.ggr` degrade dosyasından alır.
- Doğrusal, konik ve kare degradelerin **açısı**, [[docker.edge_pad]] (her
  ucun ne kadarının düz renk olduğu) ve dairesel, konik ve kare degradelerin
  merkez kayması.
- [[docker.repeat]]: [[docker.repeat_none]], [[docker.repeat_sawtooth]]
  (renkler baştan yeniden akar) veya [[docker.repeat_mirror]] (ileri geri
  akarlar), [[docker.repeat_count]] sayısıyla birlikte.
- [[docker.bilinear]]: renkler ilkinden sonuncusuna ve geri akar.
- [[docker.invert_colours]]: renkler ters sırada akar.

Türün altındaki bir şerit, renklerin tüm akışını çizileceği gibi gösterir.

## Dolguyu Düzenle iletişim kutusu {#edit-fill}

<kbd>F11</kbd> seçimin dolgusu için [[edit_fill.title]] iletişim kutusunu
açar. Bir gradyan için önizleme, [[edit_fill.fill_picker]], tür, tekrar ve
renk bandı gösterilir: seçmek için düğüme tıklayın, taşımak için sürükleyin,
eklemek veya kaldırmak için çift tıklayın. Bandın altında düğümün rengi,
saydamlığı, konumu ve [[edit_fill.midpoint]] ile [[edit_fill.reverse]]
bulunur. [[edit_fill.transform]] döndürme, merkez ve kenar boşluğunu içerir.
**Tamam** değiştirdiklerinizi [[edit_fill.overprint]] ile uygular. İletişim kutusu açıkken seçim, dolguyu siz değiştirdikçe gösterir; **İptal** veya <kbd>Esc</kbd> eski dolguyu geri getirir.

![Dolguyu Düzenle iletişim kutusu](../../img/color-fills-edit-fill.png)

## Etkileşimli Dolgu aracı {#interactive-fill}

[[tool.interactive_fill]] aracı (<kbd>G</kbd>) dolguları nesnenin üzerinde
düzenler.

1. Aracı seçin ve bir nesnenin üzerinde sürükleyin: bastığınız yerden
   bıraktığınız yere kadar doğrusal bir degrade elde edersiniz. Açıyı 15
   derecelik adımlarda tutmak için <kbd>Ctrl</kbd> tuşunu basılı tutun. Bir
   tıklama varsayılan dolguyu verir.
2. Degradeyi değiştirmek için nesnenin üzerindeki düğümleri sürükleyin:
   doğrusal bir degradenin iki kare düğümü yönünü ve uzunluğunu değiştirir;
   dairesel, konik veya kare bir degradenin düğümü merkezini taşır.
3. Yeniden renklendirmek için bir palet rengini bir düğümün üzerine, oraya
   renk eklemek için düğümler arasındaki çizginin üzerine sürükleyin.

Özellik çubuğunda dolgu türü ve ayarları bulunur: düz bir dolgunun rengi; bir
degrade için tür, başlangıç ve bitiş renkleri, açı ve kenar payı.

## Alan Dolgusu aracı {#area-fill}

[[tool.area_fill]] aracı, üst üste binen birkaç nesnenin oluşturduğu bir alan
bile olsa, herhangi bir kapalı alanı o alandan yeni bir nesne oluşturarak
doldurur. Çevresindeki nesneler olduğu gibi kalır.

1. [[tool.area_fill]] aracını seçin.
2. Özellik çubuğunda dolguyu ve anahattı seçin: **Belirt** (yanındaki renk
   ve anahat için kalınlık), **Varsayılanı kullan** (yeni nesnelerin dolgusu ve
   anahattı; bu dolgu yoksa alan görünsün diye bir renk) veya **Yok**.
3. Alanın içine tıklayın.

Yeni nesne etkin katmanın en üstüne gider ve seçilir. Bir anahattın üzerine
veya hiçbir şeyin çevrelemediği bir alana tıklamak hiçbir şey oluşturmaz.

## Ağ dolguları {#mesh-fill}

Ağ dolgusu, bir nesnenin üzerinde her birinin kendi rengi ve saydamlığı olan
düğümlerden oluşan bir ızgaradır; renkler düğümler arasında düzgünce akar.
Gerçekçi gölgelendirmeye uygundur.

1. [[tool.mesh_fill]] aracını (<kbd>M</kbd>) seçin ve özellik çubuğunda
   satırları ve sütunları ayarlayın (başlangıçta 2'ye 2, en çok 50).
2. Bir nesneye tıklayın. Nesne, dolgusunun renginde bir ağ alır.
3. Seçmek için bir düğüme, ardından onu renklendirmek için paletteki bir
   renge tıklayın. Renklerin akışını bükmek için düğümleri sürükleyin.
4. En yakın düğümden geçen bir satır veya sütun eklemek için ağın içine çift
   tıklayın; satırını ve sütununu kaldırmak için bir düğüm seçin ve
   <kbd>Delete</kbd> tuşuna basın.

Özellik çubuğunda ayrıca renkleri daha yumuşak karıştıran **Düzgün renk**,
seçili düğümün **saydamlığı** ve ağ dolgusunu kaldıran **Ağı Temizle**
bulunur. Bir ağın satırlarını ve sütunlarını değiştirmek renklerini korur.

## Desen ve doku dolguları {#pattern-texture}

- [[docker.fill_pattern]] dolgusu bir karoyu yineler: iki renk, bir vektör
  deseni veya bir bitmap. Bkz. [Desen dolguları](patterns.md).
- [[docker.fill_texture]] dolgusu üretilmiş, doğal görünümlü bir dokudur:
  renkleri ve ölçeğiyle birlikte [[texture.clouds]], [[texture.marble]],
  [[texture.noise]] veya [[texture.wood]].

## Bir nesnede birden çok dolgu {#several-fills}

[[docker.properties]] panelinin dolgu bölümünde [[docker.add_fill]],
nesnenin kendi dolgusunun üzerine, değiştirilmek üzere bir kopya olarak
başlayan başka bir dolgu boyar. Sonraki dolgular "Dolgu 2", "Dolgu 3" gibi
görünür; her birinin kendi düzenleyicisi ve onu kaldıran bir düğmesi vardır.
Anahat bölümü de aynı şekilde başka anahatlar ekleyebilir.

Dolgular anahatlara da boyanabilir; bkz.
[Anahatları ayarlama](../lines-shapes/outlines.md#outline-pen).
