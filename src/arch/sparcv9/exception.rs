use core::arch::asm;
use core::fmt;
use crate::serial_println;

// -----------------------------------------------------------------------------
// HARİCİ MONTAJ DİLİ İŞLEYİCİLERİ
// -----------------------------------------------------------------------------

// SPARC V9'da her tuzak tipi için TBR + (TID * 0x40) adresinde bir vektör bulunur.
extern "C" {
    /// 0x00: Reset, Genişletilmiş Reset, Yazılım Tuzağı 0 (Genellikle Reset Vektörü)
    fn vector_trap_reset();
    /// 0x40: Talimat Erişim Hataları (ITLBMISS/ITLBHIT)
    fn vector_trap_instruction_access();
    /// 0x60: Sistem Çağrısı (SYSCALL)
    fn vector_trap_syscall();
    /// 0x80: Veri Erişim Hataları (DTLBMISS/DTLBHIT)
    fn vector_trap_data_access();
    /// 0x100: Harici Kesme (IRQ)
    fn vector_trap_interrupt();
}

// -----------------------------------------------------------------------------
// 1. TUZAK KAYIT YAPILARI
// -----------------------------------------------------------------------------

/// SPARC V9'da tuzak (trap) oluştuğunda yığına kaydedilen CPU durumu (Context).
/// SPARC Register Pencereleri kullandığından, istisna işleyici R-Penceresini değiştirir
/// ve yığına sadece kritik GPR'ları ve kontrol yazmaçlarını kaydeder.
#[repr(C)]
pub struct ExceptionContext {
    // Montaj kodunun kaydettiği tüm genel amaçlı registerlar (g1-g7) + (o0-o7)
    // SPARC'ın Window yapısı nedeniyle bu biraz karmaşıktır.
    // Temsili olarak Global Registerları ve kritik kontrol yazmaçlarını ekliyoruz:
    pub gpr_g: [u64; 7], // g1-g7 (g0=zero, g7=thread/asi)
    pub gpr_o: [u64; 8], // o0-o7 (o6=SP)
    
    // Kontrol ve Durum Yazmaçları
    pub tstate: u64, // Tuzak Durum Yazmacı (Trap State Register)
    pub tba: u64, // Tuzak Temel Adresi Yazmacı (TBA - Trap Base Address)
    pub tpc: u64, // Tuzak Program Sayacı (Trap Program Counter - Dönüş Adresi)
    pub tnpc: u64, // Tuzak Sonraki Program Sayacı (Trap Next Program Counter)
    pub can_restore: u64, // Kayıt Pencere Sayısı
}

/// Tuzak Tipleri (TID - Trap Identification Number)
#[repr(u64)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TrapType {
    Reset = 0x00,               // Reset veya Genişletilmiş Reset
    InstructionAccessException = 0x04, // Talimat TLB Miss/Hata
    Syscall = 0x20,             // Yazılım Tuzağı / SYSCALL
    DataAccessException = 0x28,  // Veri TLB Miss/Hata
    Interrupt = 0x100,          // Harici Kesme (IRQ)
    _Unknown,
}

// -----------------------------------------------------------------------------
// 2. TEMEL İŞLEYİCİ FONKSİYONLARI (Rust Kodu)
// -----------------------------------------------------------------------------

/// Tüm tuzak giriş noktalarından montaj kodu tarafından çağrılan Rust işleyicisi.
///
/// # Parametreler
/// * `trap_id`: Tuzak Kimlik Numarası (TID)
#[no_mangle]
pub extern "C" fn generic_trap_handler(trap_id: u64, context: &mut ExceptionContext) {
    let trap_offset = trap_id * 0x40; // TID * Vektör Boyutu (0x40 bayt)

    let cause = match trap_offset {
        0x00 => TrapType::Reset,
        0x40 => TrapType::InstructionAccessException,
        0x80 => TrapType::DataAccessException,
        0x100 => TrapType::Interrupt,
        0x60 => TrapType::Syscall,
        _ => TrapType::_Unknown,
    };
    
    match cause {
        TrapType::Interrupt => {
            // Harici Kesme İşleyicisi
            handle_interrupt(context);
        }
        TrapType::InstructionAccessException | TrapType::DataAccessException => {
            // MMU/Sayfa Hatası İşleyicileri
            serial_println!("\n--- TLB/SAYFA HATASI ---");
            serial_println!("Neden: {:?}", cause);
            // SPARC'ta hatalı adresı, istisna sırasında özel MMU yazmaçlarından (MMU_HETAG, vb.) 
            // veya TPC/TNPC'den okumanız gerekir.
            serial_println!("Hata Adresi: Okunmalı (MMU CSR)");
            serial_println!("TPC (RIP): {:#x}", context.tpc);
            
            panic!("Kritik MMU Hatası!");
        }
        TrapType::Syscall => {
            // Sistem Çağrısı
            serial_println!("SYSCALL: Çağrı kodu: {}", context.gpr_o[0]); // o0 (r8)
            
            // Syscall'dan dönmek için TPC/TNPC'yi ayarlamanız GEREKİR.
            // İşlem tamamlandıktan sonra TPC = TNPC olmalıdır.
            context.tpc = context.tnpc; 
        }
        _ => {
            // Diğer kritik tuzaklar
            serial_println!("\n--- KRİTİK TUZAK ---");
            serial_println!("TID: {:#x} ({:?})", trap_id, cause);
            serial_println!("TPC: {:#x}", context.tpc);
            serial_println!("TSTATE: {:#x}", context.tstate);
            
            panic!("İşlenmemiş Tuzak!");
        }
    }
}

/// Donanım Kesmeleri (IRQ) için özel işleyici.
fn handle_interrupt(_context: &mut ExceptionContext) {
    // 1. PIC (Sun4u/UPA) adresinden hangi IRQ'nun geldiğini oku.
    
    // 2. Uygun sürücüyü çağır.

    // serial_print!("@");

    // 3. Kesmenin bittiğini (EOI) Kesme Kontrolcüsüne bildir (GEREKLİ).
    // unsafe { arch::sparcv9::upa::send_eoi(); }
}


// -----------------------------------------------------------------------------
// 3. TUZAK YÖNETİMİ API'SI
// -----------------------------------------------------------------------------

/// Tuzak Vektör Tablosunu kurar ve kesmeleri etkinleştirir.
pub fn init_exceptions() {
    unsafe {
        // 1. TBR (Trap Base Register) yazmacını kur.
        // Vektör tablosu 4KB'ta hizalanmalıdır.
        // Linker script'te 0x0000_0000_0010_0000 (Temsili) adresine yerleştirdiğimizi varsayalım.
        const TRAP_BASE_ADDRESS: u64 = 0x0000_0000_0010_0000;
        
        // Bu montaj kodunun, tüm vektör işleyicilerini (vector_trap_reset, vb.)
        // bu adresten başlayarak doğru ofsetlere yerleştirdiği varsayılır.

        // TBR'ye yazma (TBR: ccr 0, trap base register)
        asm!("mov %g0, %l0; mov {}, %l0; sethi %hi(0), %tbr; or %l0, %lo(0), %tbr", 
             in(reg) TRAP_BASE_ADDRESS, 
             out("l0") _, 
             options(nostack, nomem)); // Basit bir mtspr/mtc0 simülasyonu

        // 2. PSTATE (Processor State Register) yazmacını ayarla (Kesmeleri etkinleştirme).
        
        let mut tstate: u64;
        // TSTATE'i oku
        asm!("rdpr %tstate, {}", out(reg) tstate); 

        // IEL (Interrupt Enable Level) maskesini ayarla (örn: 1-15 seviye kesmeleri etkinleştir)
        // Burada basitçe PIE (Previous Interrupt Enable) bitini ayarlayalım.
        // TSTATE'deki IE bitini (bit 17) ayarla.
        tstate |= 1 << 17; 
        
        // TSTATE'e yaz
        asm!("wrpr {}, %tstate", in(reg) tstate); 
    }
    
    serial_println!("[SPARCV9] Tuzak Yönetimi başlatıldı.");
    serial_println!("[SPARCV9] Harici kesmeler (IE) etkinleştirildi.");
}