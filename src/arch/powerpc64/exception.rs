use core::arch::asm;
use core::fmt;
use crate::serial_println;

// -----------------------------------------------------------------------------
// HARİCİ MONTAJ DİLİ İŞLEYİCİLERİ
// -----------------------------------------------------------------------------

// PowerPC'de her istisna (exception) sabit bir vektör adresine sıçrar. 
// Montaj kodu, bu adreslere yerleştirilmelidir (genellikle 0x100, 0x200, 0x300, ...).
extern "C" {
    /// 0x100: Makine Kontrol Programı Kesmesi (Machine Check Interrupt - MCP)
    fn vector_mcp();
    /// 0x200: Sistem Yazılımı Tuzağı (System Software Trap)
    fn vector_system_software_trap();
    /// 0x300: Dış Kesme (Harici IRQ)
    fn vector_external_interrupt();
    /// 0x500: Veri Depolama Kesmesi (Data Storage Interrupt - Sayfa Hatası Yükleme/Depolama)
    fn vector_data_storage();
    /// 0x600: Talimat Depolama Kesmesi (Instruction Storage Interrupt - Sayfa Hatası Talimat)
    fn vector_instruction_storage();
    /// 0xC00: Sistem Çağrısı (SYSCALL)
    fn vector_syscall();
    /// 0xD00: Program Kontrol Kesmesi (Program Interrupt - Geçersiz Talimat vb.)
    fn vector_program_interrupt();
}

// -----------------------------------------------------------------------------
// 1. İSTİSNA KAYIT YAPILARI
// -----------------------------------------------------------------------------

/// PowerPC 64'te istisna oluştuğunda yığına kaydedilen CPU durumu (Context).
/// Bu yapının düzeni, montaj kodunun registerları yığına kaydettiği sıraya UYMALIDIR.
#[repr(C)]
pub struct ExceptionContext {
    // Montaj kodunun kaydettiği tüm genel amaçlı registerlar (r1-r31)
    // r1, r2 (SP, RTOC) dahil edilmezse 30 GPR
    pub gpr: [u64; 30], 
    
    // Kritik Kontrol Yazmaçları
    pub srr0: u64, // İstisna Dönüş Adresi (SRR0)
    pub srr1: u64, // Kayıtlı Makine Durumu (SRR1 - MSR kopyası)
    pub xer: u64, // Sabit Sayı İstisna Kaydı (Fixed Point Exception Register)
    pub ctr: u64, // Sayaç Kaydı (Count Register)
    pub lr: u64, // Bağlantı Kaydı (Link Register)
    pub dar: u64, // Veri Adres Kaydı (Data Address Register - Sayfa Hatası Adresi)
    pub dsisr: u64, // Veri Depolama İstisna Durum Kaydı (DSISR)
    pub sp: u64, // Yığın İşaretçisi (r1)
}

/// Temel istisna tipleri (Vektör adreslerine göre).
#[repr(u64)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ExceptionType {
    MCP = 0x100,        // Makine Kontrol Programı
    SysSoftTrap = 0x200, // Sistem Yazılımı Tuzağı
    ExternalInt = 0x300, // Harici Kesme (IRQ)
    DataStorage = 0x500, // Veri Depolama (Sayfa Hatası Yükleme/Depolama)
    InstructionStorage = 0x600, // Talimat Depolama (Sayfa Hatası Talimat)
    Syscall = 0xC00,    // Sistem Çağrısı
    ProgramInt = 0xD00,  // Program Kesmesi
    _Unknown,
}

// -----------------------------------------------------------------------------
// 2. TEMEL İŞLEYİCİ FONKSİYONLARI (Rust Kodu)
// -----------------------------------------------------------------------------

/// Tüm istisna giriş noktalarından montaj kodu tarafından çağrılan Rust işleyicisi.
///
/// # Parametreler
/// * `vector_offset`: İstisnanın geldiği vektör adresinin ofseti (0x100, 0x300, vb.)
#[no_mangle]
pub extern "C" fn generic_exception_handler(vector_offset: u64, context: &mut ExceptionContext) {
    let cause = match vector_offset {
        0x100 => ExceptionType::MCP,
        0x300 => ExceptionType::ExternalInt,
        0x500 => ExceptionType::DataStorage,
        0x600 => ExceptionType::InstructionStorage,
        0xC00 => ExceptionType::Syscall,
        0xD00 => ExceptionType::ProgramInt,
        _ => ExceptionType::_Unknown,
    };
    
    match cause {
        ExceptionType::ExternalInt => {
            // Harici Kesme İşleyicisi
            handle_interrupt(context);
        }
        ExceptionType::DataStorage | ExceptionType::InstructionStorage => {
            // Sayfa Hatası İşleyicileri
            serial_println!("\n--- SAYFA HATASI ---");
            serial_println!("Neden: {:?}", cause);
            serial_println!("Hata Adresi (DAR): {:#x}", context.dar);
            serial_println!("RIP (SRR0): {:#x}", context.srr0);
            
            // DSISR'ı kontrol ederek hatanın tipini (yazma/okuma) belirle.
            panic!("Kritik Sayfa Hatası!");
        }
        ExceptionType::Syscall => {
            // Sistem Çağrısı
            serial_println!("SYSCALL: Çağrı kodu: {}", context.gpr[0]); // r3'te olması gerekir, r0'ı kullanıyoruz
            
            // Syscall'dan dönmek için SRR0'ı bir sonraki talimata ilerletmeniz GEREKİR.
            context.srr0 += 4; // PPC64 talimatları 4 bayttır.
        }
        _ => {
            // Diğer kritik senkron istisnalar
            serial_println!("\n--- KRİTİK İSTİSNA ---");
            serial_println!("Vektör: {:#x} ({:?})", vector_offset, cause);
            serial_println!("RIP (SRR0): {:#x}", context.srr0);
            serial_println!("MSR (SRR1): {:#x}", context.srr1);
            
            panic!("İşlenmemiş İstisna!");
        }
    }
}

/// Donanım Kesmeleri (IRQ) için özel işleyici.
fn handle_interrupt(_context: &mut ExceptionContext) {
    // 1. PIC (MPIC/PIM) MMIO adresinden hangi IRQ'nun geldiğini oku.
    
    // 2. Uygun sürücüyü çağır.

    // serial_print!("^"); // Kesme geldiğini göstermek için

    // 3. Kesmenin bittiğini (EOI) PIC'e bildir (GEREKLİ).
    // unsafe { arch::powerpc64::mpic::send_eoi(); }
}


// -----------------------------------------------------------------------------
// 3. KESME YÖNETİMİ API'SI
// -----------------------------------------------------------------------------

/// İstisna vektörlerini kurar ve kesmeleri etkinleştirir.
pub fn init_exceptions() {
    unsafe {
        // 1. Vektör İşleyicileri (Montaj Kodu) sabit adreslerde (0x000...0x1000)
        //    bulunmalıdır. Bu, bağlayıcı (linker) komut dosyası tarafından sağlanır.
        
        // 2. MSR (Machine State Register) yazmacını ayarla (Kesmeleri etkinleştirme).
        // MSR'nin bitleri, harici kesme (EE), makine kontrol kesmesi (ME) gibi 
        // durumları kontrol eder.
        
        let mut msr: u64;
        // MSR yazmacını oku
        asm!("mfsrr1 {}", out(reg) msr); // Veya mfsr rX, MSR

        // EE (External Interrupt Enable) bitini ayarla
        msr |= 1 << 16; 
        
        // MSR yazmacına yaz
        asm!("mtsrr1 {}", in(reg) msr); // Veya mtsr MSR, rX
        
        // Bu kod, çekirdeğin halihazırda uygun izin seviyesinde (Supervisor/Hypervisor)
        // çalıştığını varsayar.
    }
    
    serial_println!("[PPC64] İstisna Yönetimi başlatıldı.");
    serial_println!("[PPC64] Harici kesmeler (EE) etkinleştirildi.");
}