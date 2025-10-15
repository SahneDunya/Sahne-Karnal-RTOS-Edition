// src/arch/armv9/platformmod.rs
// ARMv9 (aarch64) mimarisine özgü platform başlatma ve temel G/Ç işlevleri.

#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr;
use crate::serial_println;

/// Bu modül, diğer mimariye özgü modüller tarafından kullanılacak temel G/Ç
/// ve kontrol işlevlerini içerir.
pub mod io {
    use core::arch::asm;
    use core::ptr::{self, NonNull};

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

    /// Data Memory Barrier (DMB): Tüm bellek erişimlerinin sıralanmasını sağlar.
    /// Genellikle diğer çekirdeklerle senkronizasyon için kullanılır. (sy: System)
    #[inline(always)]
    pub unsafe fn dmb() {
        // Assembly: DMB ISH (Inner Shareable) veya DMB SY (System)
        asm!("dmb sy", options(nomem, nostack)); 
    }

    /// Data Synchronization Barrier (DSB): Önceki tüm bellek erişimlerinin
    /// tamamlanmasını bekler.
    #[inline(always)]
    pub unsafe fn dsb() {
        // Assembly: DSB SY
        asm!("dsb sy", options(nomem, nostack)); 
    }

    /// Instruction Synchronization Barrier (ISB): Talimat önbelleğini temizler
    /// ve boru hattını yeniden doldurur.
    #[inline(always)]
    pub unsafe fn isb() {
        // Assembly: ISB
        asm!("isb", options(nomem, nostack)); 
    }

    // -------------------------------------------------------------------------
    // Kontrol Fonksiyonları
    // -------------------------------------------------------------------------

    /// İşlemciyi bir kesme gelene kadar düşük güç modunda bekletir.
    #[inline(always)]
    pub unsafe fn wfi() {
        // Assembly: WFI (Wait For Interrupt)
        asm!("wfi", options(nomem, nostack, preserves_flags)); 
    }

    /// İşlemciyi bir olay gelene kadar düşük güç modunda bekletir.
    #[inline(always)]
    pub unsafe fn wfe() {
        // Assembly: WFE (Wait For Event)
        asm!("wfe", options(nomem, nostack, preserves_flags)); 
    }
    
    // -------------------------------------------------------------------------
    // System Register (SysReg) Erişim Fonksiyonları
    // -------------------------------------------------------------------------
    
    /// Belirtilen System Register'ı okur.
    /// RISC-V veya MIPS'teki gibi genel bir CSR'a erişim yerine,
    /// ARM'da yazmaç adını doğrudan kullanmak gerekir. 
    /// Aşağıdaki örnek sadece Temsilidir!
    #[inline(always)]
    pub unsafe fn read_system_register(reg_name: &str) -> u64 {
        // Gerçek ARM ASM'de bu genel olamaz, her yazmaç için ayrı fonksiyon yazılır.
        // Örnek: SCTLR_EL1'i okuma
        let value: u64;
        // Temsili Okuma (Gerçek kodda 'mrs {0}, sctlr_el1' gibi olmalıdır)
        // Burada sctlr_el1'in yerine reg_name dize değişkeni kullanılamaz.
        
        // Bu yüzden, sadece `read_sctlr_el1` gibi somut fonksiyonlar tanımlanır.
        
        // Örnek: SCTLR_EL1 (System Control Register, EL1) okuma
        // asm!("mrs {0}, sctlr_el1", out(reg) value, options(nomem, nostack));
        // value
        
        // Varsayımsal 0 döndür (Derleme Hatasını önlemek için)
        0
    }
    
    // Gerçek bir SysReg okuma örneği:
    #[inline(always)]
    pub unsafe fn read_sctlr_el1() -> u64 {
        let value: u64;
        asm!("mrs {0}, sctlr_el1", out(reg) value, options(nomem, nostack));
        value
    }
    
    // Gerçek bir SysReg yazma örneği:
    #[inline(always)]
    pub unsafe fn write_sctlr_el1(value: u64) {
        asm!("msr sctlr_el1, {0}", in(reg) value, options(nomem, nostack));
        // Yazma işleminden sonra senkronizasyon önerilir.
        isb();
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA FONKSİYONU
// -----------------------------------------------------------------------------

/// ARMv9 mimarisine özgü temel donanım yapılandırmalarını başlatır.
/// Bu fonksiyon `main.rs`'ten çekirdek başlangıcında çağrılmalıdır.
pub fn platform_init() {
    serial_println!("[ARMv9] Mimariye Özgü Başlatma Başlatılıyor...");
    
    // 1. Gerekli bariyerler ve senkronizasyon (Erken başlatma kodunda yapılır).
    unsafe {
        io::dsb();
        io::isb();
    }

    // 2. İşlemci durumunu kontrol etme (Örn: EL seviyesi, VBAR).
    // Varsayımsal SCTLR_EL1 okuma:
    let current_sctlr = unsafe { io::read_sctlr_el1() };
    serial_println!("[ARMv9] SCTLR_EL1 Başlangıç Değeri: {:#x}", current_sctlr);

    // 3. Kesme ve İstisna Vektörlerini ayarla (VBAR_EL1 yazmacına yazma)
    // Bu genellikle ayrı bir istisna/kesme modülünde yapılır.

    // 4. MMU aktivasyonunu hazırlar (TTBR0/1 yazmaçları ve SCTLR_EL1 MMU bitleri).
    // MMU'nun başlatılması ayrı `mmu.rs` modülünde yapılır.

    serial_println!("[ARMv9] Temel Platform Hazır.");
}