# Saydamlık

Saydamlık, bir nesnenin altında kalanın onun içinden görünmesini sağlar. Her
yerde aynı olabilir, nesne boyunca bir degrade gibi değişebilir veya bir
deseni ya da dokuyu izleyebilir.

## Saydamlık uygulama {#apply}

- [[tool.transparency]] aracıyla: bir nesneye tıklayın, ardından özellik
  çubuğunda bir saydamlık türü seçin veya opaktan (bastığınız yer) saydama
  (bıraktığınız yer) uzanan bir degrade saydamlık için nesnenin üzerinde
  sürükleyin. Degradenin düğümleri ardından [[tool.interactive_fill]] aracındaki gibi nesnede görünür: başlangıcı, sonu veya yuvarlak eğim düğümünü sürükleyin; iki düğüm arasındaki çubuğu (orta nokta) sürükleyerek geçişin yarıya ulaştığı yeri değiştirin; çizgiye çift tıklamak düğüm ekler, düğüme çift tıklamak onu kaldırır.
- [[docker.properties]] panelinin [[docker.transparency]] bölümünde.

## Ayarlar {#settings}

- Tür: [[transparency.none]], [[transparency.uniform]] (başlangıçta),
  [[transparency.fountain]] (degrade dolgulardaki gibi doğrusal, dairesel,
  konik, kare veya şekilli), [[transparency.pattern]] veya
  [[transparency.texture]].
- Miktar: %0 ile %100 arası (başlangıçta %50). Bir degrade için duraklar gri
  tonlarıdır: beyaz opak, siyah tamamen saydamdır.
- [[transparency.merge_mode]]: renklerin alttakiyle nasıl karıştığı. Normal
  (başlangıçta) yalnızca içinden gösterir; Ekle, Çıkar, Fark, Çarp, Böl, Daha
  açıksa, Daha koyuysa, Ton, Doygunluk, Renk ve Ters Çevir gibi diğerleri
  renkleri başka yollarla birleştirir.
- [[transparency.target_fill]], [[transparency.target_outline]] veya
  [[transparency.target_all]] öğesine uygulanıp uygulanmayacağı.
- **Dondur**: alttaki nesneler taşınsa bile içinden görüneni şu anki hâliyle
  tutar.

Kaldırmak için özellik çubuğunun sonundaki [[toolbar.clear_transparency]] düğmesine tıklayın (ya da tür olarak [[transparency.none]] seçin).

Başka bir nesnenin saydamlığını kopyalamak için
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]]
komutunu kullanın. Bir nesnenin parlaklığıyla tanımlanan saydam alanlar için
bkz. [Opaklık maskeleri](../objects/clip-frames.md#opacity-mask).
