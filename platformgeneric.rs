#![allow(dead_code)] // Geliştirme aşaması için uyarıları gizler

/// İşletim Sistemi Seviyesi Hata Kodları için genel bir Enum.
///
/// Statik tabanlı sistemler için önemli olan deterministik hata yönetimi sağlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    /// İşlem başarılı bir şekilde tamamlandı.
    Success,
    /// Kaynak geçici olarak kullanılamıyor (Örn: IPC kuyruğu dolu).
    ResourceBusy,
    /// Geçersiz bir parametre veya argüman sağlandı.
    InvalidArgument,
    /// İstenen kaynak bulunamadı (Örn: Statik Görev ID'si geçersiz).
    NotFound,
    /// Sistem belleği sınırlarına ulaşıldı (Statik bellekte aşım).
    OutOfMemoryStatic,
    /// Donanım veya mimariye özgü bir hata oluştu.
    PlatformSpecificError(u32),
    /// Genel, tanımlanmamış hata.
    GenericFailure,
}

/// Tüm sistem sabitlerini içeren bir yapı.
/// Bu sabitler genellikle `build.rs` veya derleme parametreleri ile ayarlanabilir.
pub struct SystemConstants;

impl SystemConstants {
    /// Maksimum görev sayısını tanımlar. Statik görev tabanlı sistemler için kritiktir.
    pub const MAX_TASKS: usize = 32;

    /// Temel zamanlayıcı tik periyodu (saniyenin kaçta biri).
    /// Sert Gerçek Zamanlı sistemler için kritik bir ayardır.
    pub const TIMER_TICK_HZ: u64 = 1000; // 1000 Hz = 1 ms periyot
    
    /// IPC mesaj kuyruğu için varsayılan derinlik.
    pub const DEFAULT_IPC_QUEUE_DEPTH: usize = 8;

    /// Çekirdek log seviyesini belirler (0: Kapalı, 1: Hata, 2: Uyarı, 3: Bilgi).
    pub const KERNEL_LOG_LEVEL: u8 = 3;
}

// -----------------------------------------------------------------------------
// GENEL YARDIMCI FONKSİYONLAR
// -----------------------------------------------------------------------------

/// İki işaretsiz tam sayının güvenli bir şekilde toplanmasını sağlar.
///
/// Gömülü sistemlerde taşma (overflow) hatalarını yönetmek önemlidir.
///
/// # Dönüş Değeri
/// İşlem başarılıysa `Some(toplam)`, taşma varsa `None`.
pub fn safe_add_usize(a: usize, b: usize) -> Option<usize> {
    a.checked_add(b)
}

/// Verilen `u64` zamanlayıcı tik sayısını milisaniye cinsinden döndürür.
///
/// # Parametreler
/// * `ticks`: `TIMER_TICK_HZ` bazında işlenen tik sayısı.
///
/// # Dönüş Değeri
/// Milisaniye cinsinden süre.
pub fn ticks_to_ms(ticks: u64) -> u64 {
    // 1000 Hz = 1 ms. T/1000 = ms
    ticks / (SystemConstants::TIMER_TICK_HZ / 1000)
}

/// Basit bir döngü tabanlı kilit (Spinlock) mekanizması için temel yapı.
///
/// NanoKernel'inizde kullanılacak gerçek kilitler için bir sarmalayıcı sağlar.
pub mod spinlock {
    use core::sync::atomic::{AtomicBool, Ordering};

    /// Basit, meşgul beklemeli (busy-waiting) kilit.
    pub struct Spinlock {
        locked: AtomicBool,
    }

    impl Spinlock {
        /// Kilidi açılmış (unlocked) durumda başlatır.
        pub const fn new() -> Self {
            Spinlock {
                locked: AtomicBool::new(false),
            }
        }

        /// Kilidi ele geçirir.
        ///
        /// Bu fonksiyon, kilit açılana kadar meşgul bir döngüde bekler.
        pub fn lock(&self) {
            // Test and Set operasyonu: Kilit 'false' ise 'true' yapar ve eski değeri döndürür.
            while self.locked.compare_exchange(
                false, // Beklenen değer (kilit açık)
                true,  // Yeni değer (kilit kapalı)
                Ordering::Acquire, // Başarılı olursa (Kilidi alırken)
                Ordering::Relaxed, // Başarısız olursa (Kilidi alamadı, tekrar dene)
            ).is_err() {
                // Kilit alınamadı, meşgul bekleme (busy-wait)
                // Daha verimli bir mimaride buraya 'yield' veya 'pause' komutu eklenir.
            }
        }

        /// Kilidi serbest bırakır.
        ///
        /// # Güvenlik Notu
        /// Çağıranın daha önce kilidi aldığından emin olması gerekir.
        pub fn unlock(&self) {
            self.locked.store(false, Ordering::Release);
        }
    }
}