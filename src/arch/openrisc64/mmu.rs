#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr::NonNull;
use crate::serial_println;
use super::io; // Bariyerler için io modülünü kullanacağız

// -----------------------------------------------------------------------------
// SAYFALAMA SABİTLERİ VE TİPLERİ (4K Sayfa Varsayımı)
// -----------------------------------------------------------------------------

/// Sayfa boyutu: 4 KiB
pub const PAGE_SIZE: usize = 4096;
pub const TLB_ENTRY_COUNT: usize = 64; // Temsili TLB boyutu

// Sayfa Tablosu Girişi (PTE) bayrakları (Temsili TLB EntryLo yazmacı için)
#[repr(u64)]
pub enum PageFlags {
    // OpenRISC'te bu bayraklar DTLB/ITLB Entry yazmacında doğrudan temsil edilir.
    VALID       = 1 << 0,  // Giriş geçerli
    WRITE       = 1 << 1,  // Yazılabilir (Writable)
    EXEC        = 1 << 2,  // Yürütülebilir (Executable)
    D_ACCESSED  = 1 << 3,  // Veri erişimi
    D_DIRTY     = 1 << 4,  // Veri yazıldı
    GLOBAL      = 1 << 5,  // Global
    
    // Önbellek Modu (Temsili: OpenRISC'te genellikle MMU bypass ayarı ile yapılır)
    CACHE_ENABLE = 1 << 6, // Önbellek etkin
    
    // Fiziksel Adres 12. bitten başlar
    ADDR_MASK   = 0xFFFFFFFFFFFFF000,
}

/// Basitleştirilmiş Sayfa Tablosu Girişi (PTE)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

// -----------------------------------------------------------------------------
// SPR YAZMAÇLARI (SPECIAL PURPOSE REGISTERS)
// -----------------------------------------------------------------------------

// OpenRISC'te MMU, TLB ve genel sistem kontrolü SPR yazmaçları ile yapılır.
// Bazı kritik SPR numaraları (Temsili, genellikle 0'dan başlar):
const SPR_MMUCFGR: u32   = 0x400; // MMU Yapılandırma
const SPR_DTLBLB: u32    = 0x900; // Veri TLB Look-up Base
const SPR_ITLBLB: u32    = 0x908; // Talimat TLB Look-up Base
const SPR_DTLBMR: u32    = 0x904; // Veri TLB Match Register (Sanal Adres)
const SPR_ITLBMR: u32    = 0x90C; // Talimat TLB Match Register (Sanal Adres)
const SPR_DTLBTR: u32    = 0x910; // Veri TLB Translate Register (Fiziksel Adres + Bayraklar)
const SPR_ITLBTR: u32    = 0x918; // Talimat TLB Translate Register (Fiziksel Adres + Bayraklar)

/// SPR yazmacını oku.
#[inline(always)]
unsafe fn read_spr(spr_num: u32) -> u64 {
    let value: u64;
    // OpenRISC assembly: 'l.mfspr rt, spr_num'
    asm!("l.mfspr {0}, {1}, 0", out(reg) value, in(reg) spr_num, options(nomem, nostack));
    value
}

/// SPR yazmacına yaz.
#[inline(always)]
unsafe fn write_spr(spr_num: u32, value: u64) {
    // OpenRISC assembly: 'l.mtspr spr_num, rt'
    // OpenRISC'te mtspr'den sonra msync veya isync GEREKLİDİR.
    asm!("l.mtspr {0}, {1}, 0", in(reg) spr_num, in(reg) value, options(nomem, nostack));
}

/// TLB'ye giriş yazar (DTLB/ITLB).
///
/// # Parametreler
/// * `tlb_index`: TLB'de yazılacak girişin indeksi.
unsafe fn tlb_write(tlb_index: u64) {
    // Önce DTLB Match ve Translate yazmaçlarına yazılır (yukarıdaki map_tlb_entry'de yapılacak)
    // Sonra TLB Base yazmacının ilgili alanına yazarak yüklenir.
    
    // Temsili TLB yükleme talimatı: DTLB Base yazmacını (DTLBLB) kullanarak yazma.
    // DTLBLB yazmacı TLB index'i ve VADDR/PTE gibi bilgileri içerir.
    
    // Sanal adres ile TLB'ye yazma
    // Bu, platformun donanım TLB yönetimine bağlıdır.
    // Genellikle Match ve Translate yazmaçlarını doldurduktan sonra 
    // özel bir yazma işlemi veya index'i DTLBLB'ye yazma ile yapılır.
    
    // Basitleştirme: DTLBLB'ye index'i yazıp yükleme yapıldığını varsayalım.
    write_spr(SPR_DTLBLB, tlb_index); 
    io::msync(); // Veri bariyeri
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA VE AKTİVASYON
// -----------------------------------------------------------------------------

/// Sanal adresi fiziksel adrese eşler ve TLB'ye yazar (4KiB sayfa).
///
/// # Güvenlik Notu
/// Bu fonksiyon doğrudan SPR'ları değiştirir.
pub unsafe fn map_tlb_entry(
    tlb_index: u64,
    virtual_addr: usize,
    physical_addr: usize,
    flags: u64,
) {
    // 1. Match Register'ı ayarla (Sanal Adres)
    // OpenRISC'te sanal adres ve sayfa boyutu (Page Size) bilgisi içerir.
    let match_reg_val = (virtual_addr as u64) | PAGE_SIZE as u64; // PAGE_SIZE VADDR'ın alt bitlerinde maske olarak kullanılır
    write_spr(SPR_DTLBMR, match_reg_val);
    write_spr(SPR_ITLBMR, match_reg_val);
    
    // 2. Translate Register'ı ayarla (Fiziksel Adres ve Bayraklar)
    // Fiziksel adres (üst 52 bit) ve bayraklar (alt 12 bit)
    let translate_reg_val = (physical_addr as u64) | flags;
    write_spr(SPR_DTLBTR, translate_reg_val);
    write_spr(SPR_ITLBTR, translate_reg_val);
    
    // 3. TLB'ye yaz (LBLB yazmacı aracılığıyla index'i belirtme)
    // DTLB için
    write_spr(SPR_DTLBLB, tlb_index);
    // ITLB için
    write_spr(SPR_ITLBLB, tlb_index);
    
    io::msync(); // Senkronizasyon bariyeri
}


/// Sayfalama (MMU/TLB) mekanizmasını etkinleştirir.
pub unsafe fn enable_paging() {
    serial_println!("[OR64] İlk TLB Girişleri Dolduruluyor...");
    
    // Varsayımsal Birebir Eşleme (İlk 16MB)
    let identity_mapping_size = 16 * 1024 * 1024; // 16 MB
    let flags = PageFlags::VALID as u64 
              | PageFlags::WRITE as u64 
              | PageFlags::EXEC as u64 
              | PageFlags::D_DIRTY as u64
              | PageFlags::CACHE_ENABLE as u64; 

    // Her 4KB'ı ayrı ayrı eşle
    let mut tlb_idx = 0;
    for addr in (0..identity_mapping_size).step_by(PAGE_SIZE) {
        if tlb_idx >= TLB_ENTRY_COUNT { break; } 
        
        // Sanal ve fiziksel adres aynı
        map_tlb_entry(tlb_idx as u64, addr, addr, flags);
        tlb_idx += 1;
    }
    
    serial_println!("[OR64] {} adet 4KB TLB girişi ({}MB) eşlendi.", tlb_idx, identity_mapping_size / (1024 * 1024));
    
    // MMU'yu etkinleştirme: SR yazmacındaki (System Register) DMMU ve IMMU bitleri.
    // OpenRISC'te genellikle SR (Supervisory Register, SPR 0x11) kullanılır.
    const SPR_SR: u32 = 0x11;
    let mut sr = read_spr(SPR_SR);
    
    // DMMU (Data MMU Enable) ve IMMU (Instruction MMU Enable) bitlerini ayarla
    // Bu bitler genellikle 0x11 (SR) yazmacında bulunur. (Temsili bitler)
    const SR_DME: u64 = 1 << 0; // Data MMU Enable
    const SR_IME: u64 = 1 << 1; // Instruction MMU Enable
    
    sr |= SR_DME | SR_IME; 
    
    write_spr(SPR_SR, sr);
    
    // Talimat senkronizasyonu
    // Bu, yazma işleminin boru hattını temizlemesini sağlar
    io::msync(); 

    serial_println!("[OR64] MMU (TLB) etkinleştirildi.");
}


/// Sayfalama sonrası çekirdek başlatma işlevi.
/// `main.rs` içinden çağrılmalıdır.
pub fn init_mmu() {
    unsafe {
        enable_paging();
    }
}