use core::arch::asm;
use core::fmt;
use crate::serial_println;

// -----------------------------------------------------------------------------
// HARİCİ MONTAJ DİLİ İŞLEYİCİSİ
// -----------------------------------------------------------------------------

// Bu, tüm istisnaların (kesmeler, hatalar, syscall) yakalandığı tek giriş noktasıdır.
// Montaj kodu (asm/exception_entry.s), tüm GPR'ları kaydeder, 
// durum bilgisini hazırlar ve ardından uygun Rust fonksiyonunu çağırır.

extern "C" {
    /// Tüm istisnalar için ortak giriş noktası (Montaj kodu).
    /// Bu fonksiyonun adresi, EENTRY yazmacına yazılacaktır.
    fn exception_entry();
}

// -----------------------------------------------------------------------------
// 1. İSTİSNA KAYIT YAPILARI
// -----------------------------------------------------------------------------

/// LoongArch 64'te istisna oluştuğunda yığına kaydedilen CPU durumu (Context).
/// Bu yapının düzeni, montaj kodunun registerları yığına kaydettiği sıraya UYMALIDIR.
#[repr(C)]
pub struct ExceptionContext {
    // Montaj kodunun kaydettiği tüm genel amaçlı registerlar (r0 - r31)
    // r0 (zero) genellikle kaydedilmez. r3-r7 (parametreler) vb. 
    // LoongArch'ta genellikle r1 yığın işaretçisi (SP) olduğundan dikkat edilmelidir.
    // Temsili olarak kritik kontrol yazmaçlarını ekliyoruz:
    
    // r1 - r31: Genel Amaçlı Yazmaçlar (GPR)
    pub gpr: [u64; 31], 
    
    // Kontrol ve Durum Yazmaçları (CSR)
    pub csr_era: u64, // İstisna dönüş adresi (Exception Return Address)
    pub csr_badv: u64, // Hatalı Sanal Adres (Bad Virtual Address - Sayfa Hatası vb.)
    pub csr_cause: u64, // İstisnanın nedeni
    pub csr_prid: u64, // İşlemci Kimliği
    pub csr_status: u64, // İşlemci durum yazmacı (Kesme durumu vb.)
}

/// İstisna nedenleri (CSR.CAUSE yazmacının 2-6. bitleri).
/// LoongArch Mimari Kaynakları'ndan alınmıştır.
#[repr(u64)]
#[derive(Debug, PartialEq)]
pub enum ExceptionCause {
    Int = 0, // Kesme (Hardware Interrupt)
    TlbRefill = 1, // TLB Yenileme (Load/Store/Instruction)
    TlbInvalid = 2, // TLB Geçersiz
    TlbModified = 3, // TLB Salt Okunur (Write to Read-Only)
    LoadPageFault = 4, // Yükleme Sayfa Hatası
    StorePageFault = 5, // Depolama Sayfa Hatası
    InstPageFault = 6, // Talimat Sayfa Hatası
    AddrInvalid = 7, // Adres Hizalama Hatası
    Syscall = 11, // Sistem Çağrısı (SYSCALL)
    Breakpoint = 12, // Kesme Noktası (BREAK)
    InstrInvalid = 14, // Geçersiz Talimat
    FpDisabled = 18, // FPU devre dışı
    _Unknown,
}

impl From<u64> for ExceptionCause {
    fn from(val: u64) -> Self {
        match val & 0x1F { // CAUSE yazmacındaki 5 bitlik kod (2-6)
            0  => ExceptionCause::Int,
            1  => ExceptionCause::TlbRefill,
            2  => ExceptionCause::TlbInvalid,
            3  => ExceptionCause::TlbModified,
            4  => ExceptionCause::LoadPageFault,
            5  => ExceptionCause::StorePageFault,
            6  => ExceptionCause::InstPageFault,
            7  => ExceptionCause::AddrInvalid,
            11 => ExceptionCause::Syscall,
            12 => ExceptionCause::Breakpoint,
            14 => ExceptionCause::InstrInvalid,
            18 => ExceptionCause::FpDisabled,
            _  => ExceptionCause::_Unknown,
        }
    }
}

// -----------------------------------------------------------------------------
// 2. TEMEL İŞLEYİCİ FONKSİYONLARI (Rust Kodu)
// -----------------------------------------------------------------------------

/// Tüm istisna giriş noktalarından montaj kodu tarafından çağrılan Rust işleyicisi.
#[no_mangle]
pub extern "C" fn generic_exception_handler(context: &mut ExceptionContext) {
    let cause_code = (context.csr_cause >> 2) & 0x1F;
    let cause = ExceptionCause::from(cause_code);
    
    match cause {
        ExceptionCause::Int => {
            // Kesme İşleyicisi
            handle_interrupt(context);
        }
        ExceptionCause::InstPageFault | ExceptionCause::LoadPageFault | ExceptionCause::StorePageFault => {
            // Sayfa Hatası İşleyicisi
            serial_println!("\n--- SAYFA HATASI ---");
            serial_println!("Neden: {:?}", cause);
            serial_println!("Hata Adresi (BadV): {:#x}", context.csr_badv);
            serial_println!("RIP: {:#x}", context.csr_era);
            panic!("Kritik Sayfa Hatası!");
        }
        ExceptionCause::Syscall => {
            // Sistem Çağrısı
            serial_println!("SYSCALL: Çağrı kodu: {}", context.gpr[10]); // Genellikle a0 (r10)
            // Syscall'dan dönmek için ERA'yı bir sonraki talimata ilerletmeniz GEREKİR.
            // context.csr_era += 4; // LoongArch talimatları 4 bayttır.
        }
        _ => {
            // Diğer senkron istisnalar
            serial_println!("\n--- KRİTİK İSTİSNA ---");
            serial_println!("Neden Kodu: {:#x} ({:?})", cause_code, cause);
            serial_println!("RIP (ERA): {:#x}", context.csr_era);
            serial_println!("Durum (Status): {:#x}", context.csr_status);
            
            panic!("İşlenmemiş İstisna!");
        }
    }
}

/// Donanım Kesmeleri (IRQ) için özel işleyici.
fn handle_interrupt(context: &mut ExceptionContext) {
    let pending_interrupts = context.csr_status & (context.csr_cause >> 10);
    
    // LoongArch'ta CAUSE yazmacının 10-15 bitleri donanım kesmelerini gösterir.
    if pending_interrupts != 0 {
        // 1. HT-PIC (LoongArch Kesme Kontrolcüsü) veya diğer kontrolcülerden
        //    hangi kesmenin geldiğini oku (genellikle MMIO/PIC yazmacı).
        
        // 2. Uygun sürücüyü çağır.

        // serial_print!("!");
    
        // 3. Kesmenin bittiğini Kesme Kontrolcüsüne (HT-PIC) bildir (EOI).
        // Bu, platform modülünde yapılmalıdır.
    } else {
         serial_println!("[LA64] Boş Kesme Vektörü!");
    }
}


// -----------------------------------------------------------------------------
// 3. KESME YÖNETİMİ API'SI
// -----------------------------------------------------------------------------

/// İstisna giriş noktasını ayarlar ve kesmeleri etkinleştirir.
pub fn init_exceptions() {
    unsafe {
        // 1. EENTRY yazmacını montaj dilindeki istisna giriş noktasına ayarla.
        let entry_addr = exception_entry as u64;
        asm!("csrwr {}, 0x0180", in(reg) entry_addr); // 0x0180 = EENTRY yazmacı
        
        // 2. STATUS yazmacını ayarla (Kesmeleri etkinleştirme).
        // Status yazmacının 2. biti (IE - Interrupt Enable) etkinleştirilir.
        // Diğer kritik bitler (örneğin PG - Sayfalama Etkin) de burada ayarlanabilir.
        
        let mut status: u64;
        asm!("csrrd {}, 0x0001", out(reg) status); // 0x0001 = STATUS yazmacı oku
        
        // IE (Interrupt Enable) bitini (2. bit) ayarla
        status |= 1 << 2; 
        
        asm!("csrwr {}, 0x0001", in(reg) status); // STATUS yazmacına yaz
    }
    
    serial_println!("[LA64] İstisna Giriş Noktası (EENTRY) yüklendi.");
    serial_println!("[LA64] Harici kesmeler (IE) etkinleştirildi.");
}