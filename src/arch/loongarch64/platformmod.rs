// src/arch/loongarch64/platformmod.rs
// LoongArch 64 (LA64) mimarisine özgü platform başlatma ve temel G/Ç işlevleri.

#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr;
use crate::serial_println;

/// Bu modül, diğer mimariye özgü modüller tarafından kullanılacak temel G/Ç
/// ve kontrol işlevlerini içerir.
pub mod io {
    use core::arch::asm;
    use core::ptr;

    // -------------------------------------------------------------------------
    // MMIO (Memory-Mapped I/O) Fonksiyonları
    // -------------------------------------------------------------------------

    /// Verilen bellek adresinden 8 bit (byte) okur (Volatile).
    #[inline(always)]
    pub unsafe fn read_mmio_8(addr: usize) -> u8 {
        ptr::read_volatile(addr as *const u8)
    }

    /// Verilen bellek adresine 8 bit (byte) yazar (Volatile).
    #[inline(always)]
    pub unsafe fn write_mmio_8(addr: usize, value: u8) {
        ptr::write_volatile(addr as *mut u8, value)
    }
    
    // NOT: 16, 32 ve 64 bit MMIO okuma/yazma fonksiyonları gerektiğinde eklenebilir.

    // -------------------------------------------------------------------------
    // Senkronizasyon (Bariyer) Fonksiyonları
    // -------------------------------------------------------------------------

    /// Data Barrier (dbar): Bellek işlemlerinin sıralanmasını sağlar (Write back, TLB vb. için).
    #[inline(always)]
    pub unsafe fn dbar() {
        // Assembly: dbar 0 (Genel veri bariyeri için)
        asm!("dbar 0", options(nomem, nostack)); 
    }

    /// Instruction Barrier (ibar): Talimat boru hattını temizler.
    #[inline(always)]
    pub unsafe fn ibar() {
        // Assembly: ibar 0 (Genel talimat bariyeri için)
        asm!("ibar 0", options(nomem, nostack)); 
    }
    
    /// Tam bellek ve talimat bariyeri.
    #[inline(always)]
    pub unsafe fn membar_all() {
        dbar();
        ibar();
    }

    // -------------------------------------------------------------------------
    // Kontrol Fonksiyonları
    // -------------------------------------------------------------------------

    /// İşlemciyi bir olay gelene kadar düşük güç modunda bekletir (LoongArch IDLE).
    #[inline(always)]
    pub unsafe fn idle() {
        // Assembly: idle 0 (Veya sadece 'idle')
        // Düşük güç bekleme talimatı
        asm!("idle 0", options(nomem, nostack, preserves_flags)); 
    }
    
    /// Kesmeleri devre dışı bırakır.
    /// Bunu yapmak için CSR (CRMD) yazmacı değiştirilmelidir.
    #[inline(always)]
    pub unsafe fn disable_interrupts() {
        // CRMD (Control Register Mod) yazmacını oku
        let crmd = read_csr(CSR_CRMD);
        // IE (Interrupt Enable) bitini (genellikle 2. bit) temizle
        const CRMD_IE: u64 = 1 << 2;
        write_csr(CSR_CRMD, crmd & !CRMD_IE);
    }
    
    /// Kesmeleri etkinleştirir.
    #[inline(always)]
    pub unsafe fn enable_interrupts() {
        // CRMD (Control Register Mod) yazmacını oku
        let crmd = read_csr(CSR_CRMD);
        // IE (Interrupt Enable) bitini ayarla
        const CRMD_IE: u64 = 1 << 2;
        write_csr(CSR_CRMD, crmd | CRMD_IE);
    }

    // -------------------------------------------------------------------------
    // CSR (Control and Status Register) Erişim Fonksiyonları
    // -------------------------------------------------------------------------

    // Örnek CSR Numaraları (LoongArch Mimarisine Göre Temsili)
    pub const CSR_CRMD: u32 = 0x0;   // Control Register Mod
    pub const CSR_ERA: u32 = 0x6;    // Exception Return Address
    pub const CSR_PRMD: u32 = 0x10;  // Privilege Register Mod

    /// Belirtilen CSR yazmacını okur.
    #[inline(always)]
    pub unsafe fn read_csr(csr_num: u32) -> u64 {
        let value: u64;
        // Assembly: csrrd rd, csr_num
        asm!("csrrd {0}, {1}", out(reg) value, in(reg) csr_num, options(nomem, nostack));
        value
    }

    /// Belirtilen CSR yazmacına yazar.
    #[inline(always)]
    pub unsafe fn write_csr(csr_num: u32, value: u64) {
        // Assembly: csrwr rs, csr_num
        asm!("csrwr {0}, {1}", in(reg) value, in(reg) csr_num, options(nomem, nostack));
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA FONKSİYONU
// -----------------------------------------------------------------------------

/// LoongArch 64 mimarisine özgü temel donanım yapılandırmalarını başlatır.
/// Bu fonksiyon `main.rs`'ten çekirdek başlangıcında çağrılmalıdır.
pub fn platform_init() {
    serial_println!("[LA64] Mimariye Özgü Başlatma Başlatılıyor...");
    
    // 1. Kesmeleri devre dışı bırak (Güvenlik için)
    unsafe {
        io::disable_interrupts();
    }
    serial_println!("[LA64] Kesmeler devre dışı bırakıldı.");

    // 2. Erken platform kayıtlarının ayarlanması (Örn: CRMD, PRMD)
    // CRMD'yi EL0 (User), EL1 (Supervisor), EL2 (Hypervisor) veya EL3 (Machine/Secure)
    // gibi uygun bir ayrıcalık seviyesine ayarla.
    
    let current_crmd = unsafe { io::read_csr(io::CSR_CRMD) };
    serial_println!("[LA64] Başlangıç CRMD Değeri: {:#x}", current_crmd);

    // 3. Geçerli durumun senkronize edilmesi
    unsafe {
        io::membar_all();
    }

    // 4. Diğer alt sistemleri başlat (MMU, İstisnalar, Zamanlayıcı, vb.)
    
    serial_println!("[LA64] Temel Platform Hazır.");
}