# Damlalıklarla renk ve öznitelik kopyalama

Damlalık araçları bir nesneden bir rengi veya bütün bir öznitelik kümesini
alır ve onu başka nesnelere verir. İkisi de özellik çubuğunda gösterilen iki
adımda çalışır: **seç** (örnek al) ve **uygula** (onunla boya). Bir örnekten
sonra araç uygulamaya geçer; yeniden örnek almak için seçme düğmesine
tıklayın. <kbd>Esc</kbd> örneği bırakır ve [[tool.pick]] aracına geri döner.

İşaretçi, araç örnek alırken damlalık, uygularken boya kovasıdır. Resimler bir
grubun ya da çerçevenin içinde de renklerini verir.

## Renk Damlalığı {#color-eyedropper}

1. [[tool.color_eyedropper]] aracını seçin.
2. Rengini almak için bir nesneye tıklayın: düz bir dolgunun dolgu rengi, bir
   degradenin ilk rengi veya anahat rengi. Bir bitmapte, işaretçinin
   altındaki piksellerin rengini, özellik çubuğunda ayarlanan örnek boyutu
   (1'e 1, 2'ye 2 veya 5'e 5 piksel) üzerinden ortalamasını alarak alır.
   <kbd>Shift</kbd>+tıklama önce anahat rengini alır.
3. Nesneleri o renkle doldurmak için üzerlerine tıklayın; anahatlarını
   renklendirmek için <kbd>Shift</kbd>+tıklayın (anahattı olmayan bir nesne
   ince bir anahat alır). Boş, kapalı bir şeklin içine tıklamak onu doldurur.

Özellik çubuğu örneklenen rengi gösterir.

Bir renk almak için işaretçiyi hareket ettirirken yanındaki küçük bir kutu
altındaki rengi gösterir: bir örnek, palet rengi veya spot mürekkep ise rengin
adı, çizimin renk modundaki (CMYK veya RGB) değerleri ve onaltılık değeri. Bir
renk seçicinin damlalığı da aynı kutuyu gösterir.

## Öznitelik Damlalığı {#attributes-eyedropper}

[[tool.attributes_eyedropper]] aracı bir nesneden diğerlerine birkaç
özelliği aynı anda kopyalar.

1. [[tool.attributes_eyedropper]] aracını seçin.
2. Özellik çubuğunda neyin kopyalanacağını seçin:
   - **Özellikler**: anahat, dolgu ve metin (yazı tipi, boyut ve stil;
     metnin kendisi korunur). Başlangıçta hepsi açık.
   - **Dönüşümler**: boyut, döndürme ve konum. Başlangıçta kapalı.
   - **Efektler**: saydamlık, alt gölge ve canlı efektler. Başlangıçta
     kapalı.
3. Kopyalanacak nesneye tıklayın.
4. Kopyalanacağı her nesneye tıklayın. <kbd>Shift</kbd>+tıklama bunun yerine
   başka bir nesneden örnek alır.

Her tıklama bir geri alma adımıdır.
