# Renk paletleri

Pencerenin altı boyunca uzanan renk paleti, nesneleri renklendirmenin en
hızlı yoludur. Altındaki **belge paleti** çiziminizin kullandığı renkleri
toplar. Daha fazla palet açabilir, kendi paletlerinizi oluşturabilir ve
düzenleyebilirsiniz.

## Paleti kullanma {#using}

| Eylem | Sonuç |
|---|---|
| Bir renge tıklayın | seçimi doldurur (hiçbir şey seçili değilken yeni nesnelerin dolgusunu ayarlar) |
| Bir renge sağ tıklayın | seçimin anahattını renklendirir |
| Bir renge <kbd>Ctrl</kbd>+tıklayın | onun onda birini seçimin dolgusuna karıştırır |
| Bir rengi tıklayıp basılı tutun | aralarından seçim yapmak için onun daha açık, daha koyu, daha mat ve daha parlak tonlarını gösterir |
| Bir rengi bir nesnenin üzerine sürükleyin | nesneyi doldurur veya kenara bırakıldığında anahattını renklendirir |
| Üstü çizili kutuya tıklayın | dolguyu kaldırır (sağ tıklama: anahattı) |

Uçlardaki oklar paleti kaydırır ve sağ uçtaki düğme her rengi aynı anda
gösterir. Adını ve değerlerini görmek için işaretçiyi bir rengin üzerinde
tutun. Sol uçtaki palet menüsü düğmesi palet menüsünü açar.

Palet pencerenin başka bir kenarına taşınabilir veya kayan hâle
getirilebilir: onu noktalı tutamacından sürükleyin.

Her kenarda pencerenin kenarına ya da çizimin yanına, çizim ile o taraftaki paneller veya araç kutusu arasına yerleşebilir. Sürüklerken mavi bir şerit nereye gideceğini gösterir; bir listeden yer seçmek için tutamacına sağ tıklayın.

## Belge paleti {#document-palette}

Bir nesneye uyguladığınız her renk, çizimle birlikte kaydedilen belge
paletine katılır. Damlalığı işaretçinin altındaki rengi ekler; menüsünde
**Otomatik Güncelle** (renklerin kendiliğinden eklenmesini durdurmak için),
**Seçimden Ekle**, **Belgeden Ekle**, **Rengi Sil** ve **Paleti Sıfırla**
(yalnızca çizimin hâlâ kullandığı renkleri tutar) bulunur. Üzerine renkler
veya nesneler de sürükleyebilirsiniz. Göstermek veya gizlemek için bkz.
[Özelleştirme seçenekleri](../customize/customization.md#color-palette).

## Paletler paneli {#palettes-docker}

[[docker.palettes]] paneli sahip olduğunuz paletleri, her birindeki renk
sayısıyla birlikte listeler. Paneli
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]]
ile açın.

![Paletler paneli](../../img/color-palettes.png)

- Çizim penceresinin altında göstermek için bir paleti işaretleyin.
- [[docker.edit]] onu palet düzenleyicide açar.
- [[menu.window.palette_open]] bir palet dosyası (`.tdpal`, `.gpl`, `.ase`
  veya `.aco`) açar.
- [[menu.window.palette_from_document]] ve
  [[menu.window.palette_from_selection]], çizimde veya seçimde kullanılan
  renklerden bir palet oluşturur.
- Belge paletinin renkleri altta görünür; seçimi doldurmak için birine
  tıklayın, anahattı renklendirmek için sağ tıklayın.

## Bir paleti düzenleme {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]]
palet düzenleyiciyi açar.

![Palet Düzenleyici](../../img/color-palettes-palette-editor.png)

1. Düzenlenecek [[dialog.palette]] öğesini seçin veya yeni bir palet
   başlatmak için [[dialog.new_palette]] düğmesine tıklayın.
2. [[dialog.name]] alanını yazın.
3. Seçmek için bir renge tıklayın; [[dialog.add_color]] geçerli rengi ekler
   ve [[dialog.remove_color]] seçili rengi kaldırır.
4. Dosya olarak kaydetmek için [[dialog.save_palette]] düğmesine tıklayın.
5. [[dialog.close]] düğmesine tıklayın.

Yerleşik paletler düzenlenemez; yeni bir palet oluşturun veya bir kopyasını
düzenleyin.
