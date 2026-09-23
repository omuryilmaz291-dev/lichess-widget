# Lichess Widget ♟

Alt+C ile açılıp kapanan, boyutu değiştirilebilir, her zaman üstte duran Lichess penceresi.

## Kullanıcılar için (kurulum yok)

1. [Releases](../../releases/latest) sayfasından **LichessWidget.exe** dosyasını indir.
2. Masaüstüne veya istediğin klasöre koy, çift tıkla. Bitti.

- **Alt+C**: göster / gizle (widget dışına tıklamak gizlemez)
- **Kenarlardan sürükle**: boyutu değiştir (artık çok küçültülebilir). Boyut ve konum hatırlanır.
- **📌 (sabitle)**: yeşilken pencere her zaman üstte kalır, başka yere tıklasan da gitmez. **—**: sadece başlık çubuğuna küçült / geri büyüt.
-  **↗**: lichess.org'u tarayıcıda aç, **✕**: gizle
- Saatin yanındaki tepsi ikonuna (♟) sağ tık: "Windows ile birlikte başlat", Çıkış

> Windows "bilinmeyen yayıncı" uyarısı verebilir (exe imzasız): "Ek bilgi → Yine de çalıştır".

## Geliştirici

```bash
npm install
npm start        # denemek için
npm run dist     # release/LichessWidget.exe üretir (Windows'ta)
```

GitHub'a yüklerken `v1.0.0` gibi bir tag at (`git tag v1.0.0 && git push --tags`);
Actions exe'yi kendisi derleyip Releases'e ekler.
