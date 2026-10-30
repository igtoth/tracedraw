# Bitmapleri izleme

İzleme, bir bitmapi kaliteden ödün vermeden düzenleyebileceğiniz, yeniden
renklendirebileceğiniz ve ölçekleyebileceğiniz vektör şekillere dönüştürür:
taranmış bir logo, bir eskiz, bir fotoğraf.

## Hızlı izleme {#quick-trace}

Bir bitmap seçin ve [[menu.bitmaps > menu.bitmaps.quick_trace]] komutunu
seçin. Genel seçeneklerin [[options.bitmap_tracing]] sayfasında seçilen ön
ayarı kullanır (başlangıçta Küçük Resim).

[[menu.bitmaps.repeat_last_trace]] seçili bit eşlemi iletişim kutusu olmadan son izleme gibi yeniden izler; [[menu.bitmaps.last_trace_settings]] iletişim kutusunu son izlemenin ayarlarıyla açar. Bir bit eşlemin sağ tık menüsünde de [[menu.bitmaps.centerline_trace]] ve [[menu.bitmaps.outline_trace]] ile birlikte bu komutlar vardır.

## Bitmapi İzle iletişim kutusu {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] ve
[[menu.bitmaps.centerline_trace]] ön ayarları listeler; izleme iletişim
kutusunu açmak için birini seçin.

- **Anahat izleme** renk alanlarını şekillerle doldurur:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] ve [[menu.bitmaps.trace_high_quality]].
- **Orta çizgi izleme** çizgileri açık eğrilerle izler:
  [[menu.bitmaps.trace_technical]] ve [[menu.bitmaps.trace_line_drawing]].

![Bitmapi İzle iletişim kutusu](../../img/bitmaps-tracing.png)

İletişim kutusu, boyutunu değiştirebileceğiniz ve ekranı kaplatabileceğiniz
bir penceredir. Önizleme sol tarafını doldurur, ayarlar sağdaki panelde
durur ve bir ayarı her değiştirdiğinizde iz yeniden çıkarılır: sonucu
görmek için basılacak bir şey yoktur. Önizlemenin altındaki satır izin
**eğrilerini**, **düğümlerini** ve **renklerini** sayar. Şekilleri bitmapin
üzerine, en koyusu önce olmak üzere yerleştirmek için [[dialog.ok]]
düğmesine, bitmapi olduğu gibi bırakmak için [[dialog.cancel]] düğmesine
tıklayın.

## Önizleme {#preview}

[[dialog.trace_preview_label]] listesi önizlemenin neyi göstereceğini seçer:

- [[dialog.trace_view_before_after]]: üstte bitmap, altta izi; ya da
  [[dialog.view_side_by_side]]. [[dialog.before]] yalnızca bitmapi gösterir.
- [[dialog.trace_view_large]]: yalnızca iz, önizleme kadar büyük.
- [[dialog.trace_view_wireframe]]: izin eğrileri, bitmapin üzerine ince
  çizgilerle çizilir. Listenin yanında bir [[dialog.trace_overlay]] kaydırıcısı
  belirir: bitmapin çizgilerin altında ne kadar güçlü görüneceğini, 0
  (yalnızca çizgiler) ile 100 (bitmap olduğu gibi) arasında ayarlar;
  başlangıçta 80.

Listeden sonraki düğmeler sığdırır, uzaklaştırır, yakınlaştırır ve kaydırır.
Tekerlek işaretçinin çevresinde yakınlaştırır, sürükleme resmi taşır; iki
bölme birlikte hareket eder. Saydam kısımlar dama tahtası gösterir.

## Ayarlar {#settings}

[[dialog.trace_tab_settings]] sekmesi şunları içerir:

- [[dialog.trace_type]] Anahat veya Orta çizgi ve [[dialog.trace_image_type]]
  ön ayarı. Birini seçmek alttaki kaydırıcıları o tür resme uyan değerlere
  getirir.
- [[dialog.detail]] (0 ile 100 arası): daha fazla ayrıntı daha küçük alanları
  ve daha ince çizgileri korur; daha az ayrıntı benekleri ve greni
  düzleştirir.
- [[dialog.smoothing]] (0 ile 100 arası, başlangıçta 25): daha az düğümle daha
  düzgün eğriler.
- [[dialog.corner_smoothness]] (0 ile 100 arası): daha yuvarlak köşeler.
- [[dialog.trace_remove]]: izin dışında ne bırakılacağı. **Arka plan rengi**
  arka planı resmin kenarına ulaştığı yerde kaldırır, **Resimdeki rengin
  tamamı** o renkteki her alanı kaldırır, **Hiçbiri** her şeyi izler.
  [[dialog.trace_remove_auto]] arka plan rengini resmin kenarı boyunca bulur;
  [[dialog.trace_remove_specify]] örneğin rengini kullanır; bu rengi
  değiştirebilir ya da yanındaki damlalıkla önizlemeden alabilirsiniz.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (başlangıçta
  açık) bir rengin her alanından tek bir eğri yapar; kapalıyken her ayrı
  parça kendi eğrisi olur. [[dialog.trace_remove_overlap]] her şekli kendi
  piksellerinde tutar; kapalıyken şekiller üstlerindekilerin biraz altına
  uzanır, böylece komşular arasında boşluk görünmez.
  [[dialog.trace_group_by_color]] her rengin eğrilerini kendi grubuna koyar;
  bir rengin alanları zaten birleştirilmemişse kullanılabilir.
- [[dialog.delete_original]]: iz yerleştirildiğinde bitmap silinir.

Geçerli iz türüne uymayan ayarlar gri görünür: orta çizgi izi yalnızca
ayrıntıyı ve düzgünleştirmeyi okur.

## Renkler {#colors}

[[dialog.trace_tab_colors]] sekmesi [[dialog.color_mode]] (RGB, gri tonlama
ya da siyah beyaz) ve [[dialog.trace_number_of_colors]] (2 ile 256 arası)
ayarlarını yapar ve izin renklerini her birinin eğri sayısıyla listeler. İki
ya da daha çok rengi işaretleyip [[dialog.trace_merge_colors]] düğmesine
tıklayarak onları ilk renkte birleştirin; bir rengin örneğine tıklayarak
eğrilerini başka bir renkle çizin. Resmi yeniden izleyen bir ayarı
değiştirmek, önceki ize ait olan bu düzenlemeleri unutur.

## Sıfırla, geri al ve yinele {#steps}

Ayarlar panelinin altında [[dialog.reset]] ön ayarın ayarlarını geri getirir
ve iki ok, iletişim kutusunda yapılan her değişiklikte geri ve ileri
adımlar: her kaydırıcı bırakma, liste seçimi, onay kutusu, renk düzenlemesi
ve Sıfırla bir adımdır. Geri aldıktan sonra yapılan bir değişiklik geri
alınan adımları atar.

[[dialog.trace_options]], genel seçeneklerin [[options.bitmap_tracing]]
sayfasını açar; burada hızlı iz ön ayarını ve her izin başladığı seçimleri
belirlersiniz, sonra iletişim kutusuna dönersiniz. [[dialog.help_button]]
düğmesi bu sayfayı açar.
