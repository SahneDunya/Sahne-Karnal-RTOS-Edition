// src/arch/amd64/platformmod.rs
// AMD64 (x86_64) mimarisine özgü platform başlatma ve temel G/Ç işlevleri.

#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use crate::serial_println;

/// Bu modül, diğer mimariye özgü modüller tarafından kullanılacak temel G/Ç
/// ve kontrol işlevlerini içerir.
pub mod io {
    use core::arch::asm;

    /// Verilen G/Ç portundan 8 bit (byte) okur.
    #[inline(always)]
    pub unsafe fn inb(port: u16) -> u8 {
        let value: u8;
        // Assembly: IN AL, DX
        asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack));
        value
    }

    /// Verilen G/Ç portuna 8 bit (byte) yazar.
    #[inline(always)]
    pub unsafe fn outb(port: u16, value: u8) {
        // Assembly: OUT DX, AL
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
    }
    
    // NOT: Diğer G/Ç fonksiyonları (inw, outw, inl, outl) gerektiğinde eklenebilir.

    /// İşlemciyi bir sonraki kesme gelene kadar duraklatır (düşük güç modu).
    /// Hata ayıklama döngülerinde ve panikte kullanılır.
    #[inline(always)]
    pub unsafe fn hlt() {
        // Assembly: HLT
        asm!("hlt", options(nomem, nostack));
    }
    
    /// Talimat akışını senkronize etmek için bir bekleme talimatı ekler.
    /// Spinlock'lar veya kısıtlı döngüler için ipucu sağlar.
    #[inline(always)]
    pub unsafe fn pause() {
        // Assembly: PAUSE (REPNOP)
        asm!("pause", options(nomem, nostack));
    }
    
    /// Kesmeleri devre dışı bırakır.
    #[inline(always)]
    pub unsafe fn cli() {
        // Assembly: CLI
        asm!("cli", options(nomem, nostack));
    }

    /// Kesmeleri etkinleştirir.
    #[inline(always)]
    pub unsafe fn sti() {
        // Assembly: STI
        asm!("sti", options(nomem, nostack));
    }
    
    /// Tam bir bellek bariyeri (Memory Barrier) sağlar.
    /// Tüm beklemedeki bellek operasyonlarının tamamlanmasını garantiler.
    #[inline(always)]
    pub unsafe fn membar_all() {
        // Assembly: LFENCE / MFENCE / SFENCE
        // Genellikle MFENCE, tüm bellek bariyeri için kullanılır.
        asm!("mfence", options(nostack));
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA FONKSİYONU
// -----------------------------------------------------------------------------

/// AMD64 mimarisine özgü temel donanım yapılandırmalarını başlatır.
/// Bu fonksiyon `main.rs`'ten çekirdek başlangıcında çağrılmalıdır.
pub fn platform_init() {
    serial_println!("[AMD64] Mimariye Özgü Başlatma Başlatılıyor...");
    
    // 1. Seri G/Ç doğrulama (Zaten `main.rs` veya `lib.rs` tarafından yapılmış olmalı).

    // 2. Temel işlemci durumunu kontrol etme ve ayarlama (Örn: MSR'lar).
    // Gelişmiş kodda burada GDT/IDT/TSS/CR0/CR4 ayarları yapılacaktır.
    
    // 3. Kesmeleri devre dışı bırak (Güvenlik için)
    unsafe {
        io::cli();
    }
    serial_println!("[AMD64] Kesmeler devre dışı bırakıldı (CLI).");

    // 4. Diğer alt sistemleri başlat (MMU, Zamanlayıcı, Kesme Denetleyicisi, vb.)
    // Burası sadece platformmod.rs'nin görevi değil, ancak bir başlangıç noktasıdır.
    
    serial_println!("[AMD64] Temel Platform Hazır.");
}