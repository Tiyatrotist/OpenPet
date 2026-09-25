# OpenPet Gerçek Kedi Asset Spesifikasyonu (Feline Asset Specification)

Bu doküman, OpenPet için özel kedi çizimleri, sprite sheet'leri veya animasyon paketleri hazırlamak isteyen sanatçılar ve geliştiriciler için teknik standartları ve yönergeleri içerir.

---

## 1. Genel Format ve Renk Standartları

- **Dosya Formatı:** PNG (RGBA 32-bit, alfa kanallı şeffaf arka plan)
- **Renk Profili:** sRGB
- **Arka Plan:** Tamamen şeffaf (`#00000000`). Pencereler şeffaf piksel tıklama geçirgenliği (`HTTRANSPARENT`) kullandığından, kedi dışındaki pikseller temiz olmalıdır.
- **Kare Hızı (FPS):** 8 – 12 FPS önerilir (Motor içindeki 60 FPS zamanlayıcı kareleri yumuşak bir şekilde zamanlar).

---

## 2. Desteklenen Çözünürlük Standartları

OpenPet üç temel çözünürlük seviyesini doğal olarak destekler:

| Çözünürlük Seviyesi | Kare Boyutu (Piksel) | Atlas Boyutu | Kullanım Alanı |
|---|---|---|---|
| **Low64** | 64 × 64 | 256 × 512 | Klasik Piksel Sanatı, Düşük Kaynak Tüketimi (Varsayılan Mimi) |
| **Medium128** | 128 × 128 | 512 × 1024 | Modern HD Masaüstü Ekranları (1080p, 1440p) |
| **High256** | 256 × 256 | 1024 × 2048 | 4K / HiDPI Ekranlar, Yüksek Detaylı El Çizimleri |

---

## 3. Zorunlu Gerçek Kedi Animasyon Döngüleri

Gerçek kedi davranış motoru (`openpet-behavior`) ve Kontrol Merkezi için tanımlanan temel animasyon durumları:

### A. Temel Davranışlar (Core Behaviors)
1. **`idle` (2–4 kare):**
   - Ayakta veya oturur pozisyonda sakin bekleme.
   - Hafif nefes alma, ara sıra göz kırpma, kuyruk ucu oynaması.
2. **`walk` (4–8 kare):**
   - Gerçekçi 4 ayaklı kedi yürüyüş döngüsü (çapraz adım sekansı).
3. **`run` (4–6 kare):**
   - Hızlı koşma / depar döngüsü.
4. **`sit` (2 kare):**
   - Dik oturup ekrana veya etrafa bakınma.
5. **`sleep` (2–4 kare):**
   - Kıvrılmış kedi uykusu ("donut" veya yan yatış), göğsün yavaş inip kalkması, küçük Zzz baloncuğu.
6. **`wake` (2–4 kare):**
   - Uyanma, gerinme, göz ovuşturma ve esneme.
7. **`stretch` (2–4 kare):**
   - Tipik kedi esnemesi: ön patileri öne uzatıp sırtı kamburlaştırma.

### B. Gerçek Kediye Özgü Döngüler (Feline Specifics)
8. **`purr` (2–4 kare):**
   - Keyifli mırıltı: yarı kapalı mutlu gözler, hafif titreşen göğüs/yanaklar, tatlı kuyruk kıvrılması.
9. **`knead` (4 kare):**
   - "Hamur yoğurma" (making biscuits): ön patileri sırayla bastırma, ritmik ve mutlu ifade.
10. **`zoomies` (4–6 kare):**
    - Ani gece/gündüz enerjisi: kulaklar arkada, gözler büyümüş, kabarık kuyrukla çılgınca koşturma ve zıplama.
11. **`loaf` (1–2 kare):**
    - "Kedi ekmeği" pozu: tüm patiler gövdenin altına toplanmış, derli toplu ve huzurlu duruş.
12. **`groom` (4–6 kare):**
    - Patisini yalayıp yüzünü ve kulak arkasını silme, temizlenme.
13. **`eat` (3–4 kare):**
    - Mama veya balık yerken ağız çiğneme hareketi ve keyifli ifade.
14. **`drink` (2–4 kare):**
    - Su kabından dilini çıkartarak lıkır lıkır su içme döngüsü.
15. **`play` (4–6 kare):**
    - Yumak veya oyuncakla oynama, patisiyle yuvarlama, havaya zıplama.
16. **`pet` (2–4 kare):**
    - Sevildiğinde başını veya sırtını ele sürtme, kalpler çıkarma.
17. **`surprised` (2 kare):**
    - Korkma / şaşırma: sırt kabarmış, kuyruk fırça gibi, kulaklar yatık.

---

## 4. Gerçek Kedi Irk Renk Paletleri

OpenPet aşağıdaki kedi ırklarını dahili olarak destekler:

1. **Tekir (Tabby):** Gri-kahve zemin üzerine koyu şeritler, krem göğüs, zümrüt yeşili gözler.
2. **Smokin (Tuxedo):** Parlak siyah sırt ve kafa, bembeyaz boyun/göğüs ve patiler, sarı-yeşil gözler.
3. **Calico (Alacalı):** Üç renkli mozaik yama deseni (beyaz gövde, turuncu ve siyah lekeler), kehribar gözler.
4. **Sarıman (Ginger):** Sıcak turuncu / marmelat çizgili kürk, açık krem çene, bal rengi gözler.
5. **Siyam (Siamese):** Açık krem gövde, koyu kahverengi kulak/yüz maskesi ve patiler, safir mavisi gözler.
6. **Siyah Kedi (Black):** İpeksi gece siyahı gövde, ışıltılı altın sarısı gözler.
7. **Pamuk / Beyaz (White):** Kar beyazı tertemiz kürk, parıltılı turkuaz veya heterokromik gözler.

---

## 5. Paketleme Formatı (`manifest.json`)

Kedi çizimleri `.openpet` (veya `.petpack`) uzantılı bir ZIP arşivi olarak paketlenir. Arşiv kökünde `manifest.json` ve sprite atlas görsel(ler)i yer alır:

```json
{
  "schema_version": 1,
  "id": "tekir-mimi",
  "name": "Tekir Mimi",
  "version": "1.0.0",
  "author": "Sanatçı Adı",
  "license": "CC-BY-4.0",
  "description": "El çizimi sevimli sokak tekiri animasyon paketi",
  "minimum_openpet_version": "0.1.0",
  "breed": "Tabby",
  "asset_resolution": "medium128",
  "supported_resolutions": ["low64", "medium128"],
  "atlases": ["spritesheet.png"],
  "behavior_tags": ["feline", "cat", "tabby"],
  "animations": {
    "idle": {
      "loops": true,
      "frames": [
        { "x": 0, "y": 0, "width": 128, "height": 128, "duration_ms": 125 },
        { "x": 128, "y": 0, "width": 128, "height": 128, "duration_ms": 125 }
      ]
    },
    "walk": {
      "loops": true,
      "frames": [
        { "x": 256, "y": 0, "width": 128, "height": 128, "duration_ms": 100 },
        { "x": 384, "y": 0, "width": 128, "height": 128, "duration_ms": 100 }
      ]
    },
    "sit": {
      "loops": true,
      "frames": [
        { "x": 0, "y": 128, "width": 128, "height": 128, "duration_ms": 200 }
      ]
    },
    "sleep": {
      "loops": true,
      "frames": [
        { "x": 128, "y": 128, "width": 128, "height": 128, "duration_ms": 250 },
        { "x": 256, "y": 128, "width": 128, "height": 128, "duration_ms": 250 }
      ]
    },
    "purr": {
      "loops": true,
      "frames": [
        { "x": 384, "y": 128, "width": 128, "height": 128, "duration_ms": 125 }
      ]
    },
    "drink": {
      "loops": true,
      "frames": [
        { "x": 0, "y": 256, "width": 128, "height": 128, "duration_ms": 125 }
      ]
    }
  },
  "hashes": {}
}
```

Paketlemek ve doğrulamak için dahili CLI aracı (`openpet-pack`) kullanılır:

```bash
# Paketi derle (.openpet arşivi oluştur)
cargo run -p openpet-pack -- build --source path/to/cat_assets/ --output tekir-mimi.openpet

# Paketin bütünlüğünü ve manifest güvenliğini doğrula
cargo run -p openpet-pack -- validate tekir-mimi.openpet

# Paket içeriğini ve animasyon atlaslarını incele
cargo run -p openpet-pack -- inspect tekir-mimi.openpet
```
