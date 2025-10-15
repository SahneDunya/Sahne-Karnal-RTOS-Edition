// src/arch/powerpc64/platformmod.rs
// PowerPC 64 (PPC64) mimarisine özgü platform başlatma ve temel G/Ç işlevleri.

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

    /// Data Synchronization (Memory) Barrier (DMB/sync): Bellek operasyonlarının
    /// sıralanmasını ve tamamlanmasını sağlar.
    #[inline(always)]
    pub unsafe fn sync() {
        // Assembly: sync (Data Synchronization Barrier)
        asm!("sync", options(nomem, nostack)); 
    }

    /// Instruction Synchronization Barrier (ISB/isync): Talimat önbelleğini temizler
    /// ve boru hattını yeniden doldurur.
    #[inline(always)]
    pub unsafe fn isync() {
        // Assembly: isync (Instruction Synchronization Barrier)
        asm!("isync", options(nomem, nostack)); 
    }
    
    /// Tam bellek ve talimat bariyeri.
    #[inline(always)]
    pub unsafe fn membar_all() {
        sync();
        isync();
    }

    // -------------------------------------------------------------------------
    // Kontrol Fonksiyonları
    // -------------------------------------------------------------------------

    /// İşlemciyi bir kesme gelene kadar düşük güç modunda bekletir.
    /// PowerPC'de bu, genellikle çekirdek moduna ve kullanılan talimata bağlıdır.
    #[inline(always)]
    pub unsafe fn wait() {
        // Assembly: wait (Genellikle 'wait' veya 'nap' kullanılır)
        // Eğer donanım destekliyorsa 'wait' daha verimlidir.
        asm!("wait", options(nomem, nostack, preserves_flags)); 
    }
    
    // -------------------------------------------------------------------------
    // SPR (Special Purpose Register) Erişim Fonksiyonları
    // -------------------------------------------------------------------------

    // Örnek SPR Numaraları (PowerPC Mimarisine Göre Temsili)
    pub const SPR_SRR0: u32 = 26;   // Supervisor/System Save and Restore Register 0 (Exception Return Address)
    pub const SPR_CSRR0: u32 = 58;  // Critical Save and Restore Register 0
    pub const SPR_MSR: u32 = 336;   // Machine State Register (Interrupt Enable, vb. içerir)

    /// Belirtilen SPR yazmacını okur.
    ///
    /// # Parametreler:
    /// * `spr_num`: SPR yazmacının numarası.
    #[inline(always)]
    pub unsafe fn read_spr(spr_num: u32) -> u64 {
        let value: u64;
        // Assembly: mfspr rd, spr_num (Move From SPR)
        // mfspr yazmacın numarasını (rA yazmacı yerine hemen sayısal olarak) alır.
        asm!("mfspr {0}, {1}", 
             out(reg) value, 
             in(reg) spr_num, 
             options(nomem, nostack));
        value
    }

    /// Belirtilen SPR yazmacına yazar.
    #[inline(always)]
    pub unsafe fn write_spr(spr_num: u32, value: u64) {
        // Assembly: mtspr spr_num, rs (Move To SPR)
        asm!("mtspr {0}, {1}", 
             in(reg) spr_num, 
             in(reg) value, 
             options(nomem, nostack));
    }
    
    /// Kesmeleri devre dışı bırakır (MSR yazmacı üzerinden).
    #[inline(always)]
    pub unsafe fn disable_interrupts() {
        let msr = read_spr(SPR_MSR);
        // EE (External Interrupt Enable) ve EE (Exception Enable) bitlerini temizle
        // MSR bit maskeleri mimariye göre değişir, Temsili olarak 16. biti kullanalım:
        const MSR_EE: u64 = 1 << 16; 
        write_spr(SPR_MSR, msr & !MSR_EE);
    }
    
    /// Kesmeleri etkinleştirir (MSR yazmacı üzerinden).
    #[inline(always)]
    pub unsafe fn enable_interrupts() {
        let msr = read_spr(SPR_MSR);
        const MSR_EE: u64 = 1 << 16;
        write_spr(SPR_MSR, msr | MSR_EE);
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA FONKSİYONU
// -----------------------------------------------------------------------------

/// PowerPC 64 mimarisine özgü temel donanım yapılandırmalarını başlatır.
/// Bu fonksiyon `main.rs`'ten çekirdek başlangıcında çağrılmalıdır.
pub fn platform_init() {
    serial_println!("[PPC64] Mimariye Özgü Başlatma Başlatılıyor...");
    
    // 1. Kesmeleri devre dışı bırak (Güvenlik için)
    unsafe {
        io::disable_interrupts();
    }
    serial_println!("[PPC64] Kesmeler devre dışı bırakıldı.");

    // 2. MSR yazmacının başlangıç durumunu kontrol etme
    let current_msr = unsafe { io::read_spr(io::SPR_MSR) };
    serial_println!("[PPC64] Başlangıç MSR Değeri: {:#x}", current_msr);

    // 3. Senkronizasyon
    unsafe {
        io::membar_all();
    }

    // 4. Diğer alt sistemleri başlat (MMU, İstisnalar, Zamanlayıcı, vb.)
    
    serial_println!("[PPC64] Temel Platform Hazır.");
}