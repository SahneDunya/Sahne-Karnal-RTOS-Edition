// src/arch/rv64i/platformmod.rs
// RISC-V 64 (RV64I) mimarisine özgü platform başlatma ve temel G/Ç işlevleri.

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

    /// Tam bir bellek (veri) bariyeri sağlar.
    /// RISC-V'de 'fence' talimatı kullanılır.
    /// # Parametreler: pred (öncül), succ (ardıl). 'iowr' genel bir bariyer için yaygındır.
    #[inline(always)]
    pub unsafe fn fence_all() {
        // Assembly: fence iorw, iorw
        asm!("fence iorw, iorw", options(nomem, nostack)); 
    }

    /// Talimat önbelleği (Instruction Cache) bariyeri.
    /// Talimat önbelleğini geçersiz kılar/temizler. Sayfalama tablosu güncellendikten sonra gereklidir.
    #[inline(always)]
    pub unsafe fn fence_i() {
        // Assembly: fence.i
        asm!("fence.i", options(nomem, nostack)); 
    }
    
    // -------------------------------------------------------------------------
    // Kontrol Fonksiyonları
    // -------------------------------------------------------------------------

    /// İşlemciyi bir kesme gelene kadar düşük güç modunda bekletir.
    #[inline(always)]
    pub unsafe fn wfi() {
        // Assembly: wfi (Wait For Interrupt)
        asm!("wfi", options(nomem, nostack, preserves_flags)); 
    }
    
    // -------------------------------------------------------------------------
    // CSR (Control and Status Register) Erişim Fonksiyonları
    // -------------------------------------------------------------------------

    // Örnek CSR Numaraları (Supervisor Seviyesi (S-Mode) için)
    pub const CSR_SSTATUS: u32 = 0x100; // Supervisor Status Register (Kesme etkinleştirme içerir)
    pub const CSR_SIE: u32 = 0x104;     // Supervisor Interrupt Enable Register
    pub const CSR_SATP: u32 = 0x180;    // Supervisor Address Translation and Protection (MMU)

    /// Belirtilen CSR yazmacını okur.
    #[inline(always)]
    pub unsafe fn read_csr(csr_num: u32) -> u64 {
        let value: u64;
        // Assembly: csrr rd, csr_num (Control and Status Register Read)
        // Rust'ta, CSR numarası genellikle operand yerine const olarak geçer.
        asm!("csrr {0}, {1}", out(reg) value, in(reg) csr_num, options(nomem, nostack));
        value
    }

    /// Belirtilen CSR yazmacına yazar.
    /// Genellikle `csrw` (write) veya `csrrw` (read-write) kullanılır.
    /// Burada basitlik için `csrrw` ile önceki değeri alıp, yeni değeri yazarız (bu, doğrudan yazma gibi davranır).
    #[inline(always)]
    pub unsafe fn write_csr(csr_num: u32, value: u64) {
        // Assembly: csrrw x0, csr_num, rs (x0 hedef yazmaç olarak kullanılarak önceki değer atılır)
        asm!("csrrw zero, {0}, {1}", in(reg) csr_num, in(reg) value, options(nomem, nostack));
    }
    
    /// Kesmeleri devre dışı bırakır (SSTATUS yazmacı üzerinden).
    #[inline(always)]
    pub unsafe fn disable_interrupts() {
        // SSTATUS yazmacındaki SIE (Supervisor Interrupt Enable) bitini temizle
        const SSTATUS_SIE: u64 = 1 << 1; 
        
        // Assembly: csrc (CSR Read and Clear bits)
        // SIE bitini temizler
        asm!("csrc {0}, {1}", in(reg) CSR_SSTATUS, in(reg) SSTATUS_SIE, options(nomem, nostack));
    }
    
    /// Kesmeleri etkinleştirir (SSTATUS yazmacı üzerinden).
    #[inline(always)]
    pub unsafe fn enable_interrupts() {
        const SSTATUS_SIE: u64 = 1 << 1; 
        
        // Assembly: cssr (CSR Read and Set bits)
        // SIE bitini ayarlar
        asm!("cssr {0}, {1}", in(reg) CSR_SSTATUS, in(reg) SSTATUS_SIE, options(nomem, nostack));
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA FONKSİYONU
// -----------------------------------------------------------------------------

/// RISC-V 64 mimarisine özgü temel donanım yapılandırmalarını başlatır.
/// Bu fonksiyon `main.rs`'ten çekirdek başlangıcında çağrılmalıdır.
pub fn platform_init() {
    serial_println!("[RV64I] Mimariye Özgü Başlatma Başlatılıyor...");
    
    // 1. Kesmeleri devre dışı bırak (Güvenlik için)
    unsafe {
        io::disable_interrupts();
    }
    serial_println!("[RV64I] Kesmeler devre dışı bırakıldı.");

    // 2. CSR yazmaçlarının başlangıç durumunu kontrol etme
    let current_sstatus = unsafe { io::read_csr(io::CSR_SSTATUS) };
    serial_println!("[RV64I] Başlangıç SSTATUS Değeri: {:#x}", current_sstatus);

    // 3. Senkronizasyon
    unsafe {
        io::fence_all();
    }

    // 4. Diğer alt sistemleri başlat (MMU, İstisnalar, Zamanlayıcı, vb.)
    
    serial_println!("[RV64I] Temel Platform Hazır.");
}