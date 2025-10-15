#![allow(dead_code)] // Geliştirmenin ilk aşaması için uyarıları gizler

/// Statik olarak gömülecek Initrd görüntüsünün ham verilerini tutar.
///
/// Not: Gerçek bir çekirdekte, bu veri ya bootloader tarafından
/// çekirdeğe bir adres olarak iletilir ya da build.rs ile statik olarak
/// bir byte dizisi olarak çekirdeğin içine derlenir.
/// Biz burada basitlik için derleme zamanında gömme (embedding) yöntemini kullanıyoruz.
#[cfg(target_arch = "x86_64")] // İlk hedef olarak AMD64 varsayalım
const INITRD_DATA: &[u8] = include_bytes!("../initrd.img"); // Initrd görüntünüzün yolu

/// Initrd görüntüsünün verilerine ve meta verilerine erişim sağlayan yapı.
/// Bu yapı, statik olarak belirlenmiş verilerle çalışır.
pub struct InitRd {
    data: &'static [u8],
}

impl InitRd {
    /// InitRd yapısının statik bir örneğini oluşturur ve veriye erişimi sağlar.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon, veri dizisinin var olduğunu varsayar. Eğer `INITRD_DATA`
    /// tanımlı değilse, derleme hatası oluşacaktır.
    pub fn new() -> Self {
        InitRd {
            data: INITRD_DATA,
        }
    }

    /// Initrd görüntüsünün ham bayt dizisine salt okunur erişim sağlar.
    ///
    /// # Dönüş Değeri
    /// Initrd içeriğini temsil eden statik, sabit bir bayt dilimi (`&'static [u8]`).
    pub fn get_data(&self) -> &'static [u8] {
        self.data
    }

    /// Initrd görüntüsünün toplam boyutunu (bayt olarak) döndürür.
    pub fn get_size(&self) -> usize {
        self.data.len()
    }

    /// Verilen ofset ve uzunlukta bir alt dilime (slice) erişim sağlar.
    /// Gerçek bir kullanımda, bu fonksiyon basit bir dosya sistemi okuyucusu için
    /// temel oluşturabilir.
    ///
    /// # Parametreler
    /// * `offset`: Okumaya başlanacak başlangıç konumu.
    /// * `length`: Okunacak bayt sayısı.
    ///
    /// # Dönüş Değeri
    /// İstenen alt dilim. Ofset veya uzunluk sınır dışındaysa `None` döner.
    pub fn read_slice(&self, offset: usize, length: usize) -> Option<&[u8]> {
        if offset > self.data.len() || offset.checked_add(length)? > self.data.len() {
            return None; // Sınır dışı okuma hatası
        }
        
        // slice metodu, güvenli bir şekilde alt dilimi döndürür.
        Some(&self.data[offset..offset + length])
    }

    // İleride buraya dosya arama, metadata okuma gibi fonksiyonlar eklenebilir.

}
