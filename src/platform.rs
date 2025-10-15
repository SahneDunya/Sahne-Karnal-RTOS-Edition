#![allow(dead_code)] // Geliştirme aşaması için izin verilir

/// NanoKernel'in mimariye özgü donanım soyutlamaları için Ortak Arayüz (Trait).
///
/// Tüm desteklenen platformlar (AMD64, ARMv9, RISC-V vb.) bu arayüzü uygulamalıdır.
/// Çekirdeğin geri kalanı bu trait üzerinden donanıma erişir.
pub trait Platform {
    /// İşletim sisteminin başlangıç donanım kurulumunu gerçekleştirir.
    /// Buna genellikle CPU çekirdeğinin başlatılması, temel saatlerin ayarlanması vb. dahildir.
    /// # Güvenlik Notu: Yalnızca tek çekirdekli ilk başlangıçta çağrılmalıdır.
    fn init_hardware();

    /// Verilen mesajı konsola veya hata ayıklama portuna yazdırır.
    /// Hata ayıklama ve temel çıktı için hayati öneme sahiptir.
    fn debug_print(s: &str);

    /// İşlemciyi sonsuz bir uyku döngüsüne sokar veya düşük güç moduna geçirir.
    /// Genellikle kurtarılamaz hatalardan sonra veya görev kalmadığında çağrılır.
    fn halt() -> !;

    /// Verilen bellek adresine (I/O Portu vb.) bir bayt yazar.
    /// Genellikle x86 port I/O'da kullanılır, ancak soyutlama katmanı içinde tutulur.
    unsafe fn write_byte_to_address(addr: usize, data: u8);

    /// Verilen bellek adresinden (I/O Portu vb.) bir bayt okur.
    unsafe fn read_byte_from_address(addr: usize) -> u8;

    // ... Diğer mimariye özgü ortak fonksiyonlar buraya eklenebilir
    // (Örn: set_interrupt_mask, get_core_id, timer_init)
}

// -----------------------------------------------------------------------------
// DERLEME ZAMANI MİMARİ SEÇİMİ (Conditional Compilation)
// -----------------------------------------------------------------------------

// AMD64 (x86_64) Mimarisi için:
#[cfg(target_arch = "x86_64")]
#[path = "arch/amd64/platformmod.rs"]
mod arch_platform;

// ARMv9 (aarch64) Mimarisi için:
#[cfg(target_arch = "aarch64")]
#[path = "arch/armv9/platformmod.rs"]
mod arch_platform;

// RISC-V 64 Mimarisi için (rv64i)
#[cfg(target_arch = "riscv64")]
#[path = "arch/rv64i/platformmod.rs"]
mod arch_platform;

// PowerPC 64 Mimarisi için (powerpc64)
#[cfg(target_arch = "powerpc64")]
#[path = "arch/powerpc64/platformmod.rs"]
mod arch_platform;

// SPARCv9 Mimarisi için (sparc64)
#[cfg(target_arch = "sparc64")]
#[path = "arch/sparcv9/platformmod.rs"]
mod arch_platform;


// Eksik veya henüz desteklenmeyen mimariler için bir yer tutucu (fallback)
// Bu, derleme zamanında desteklenmeyen bir mimari seçilirse hata verecektir.
#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64",
    target_arch = "powerpc64",
    target_arch = "sparc64",
    // Diğerlerini ekleyin: loongarch64, mips64, openrisc64
)))]
compile_error!("HEDEF MİMARİ DESTEKLENMİYOR: Lütfen `platform.rs` içine ekleyin.");


/// Mimariye özgü somut uygulamayı (struct) dışa aktar.
///
/// Çekirdeğin diğer modülleri, somut yapıyı çağırmak yerine
/// `platform::PlatformManager::metod()` şeklinde erişim sağlayacaktır.
/// Varsayım: Her mimari, `platformmod.rs` içinde `PlatformManager` adında
/// boş bir somut yapı (struct) tanımlar ve `Platform` trait'ini uygular.
pub use arch_platform::PlatformManager;
