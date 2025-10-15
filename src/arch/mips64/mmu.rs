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
pub const TLB_PTE_COUNT: usize = 1024; // Varsayımsal sayfa tablosu boyutu

// Sayfa Tablosu Girişi (PTE) bayrakları (MIPS EntryLo yazmacı için temsili)
#[repr(u64)]
pub enum EntryLoFlags {
    GLOBAL      = 1 << 0,  // TLB temizliğinde korunur (Global)
    VALID       = 1 << 1,  // TLB girişi geçerli
    DIRTY       = 1 << 2,  // Yazılabilir (Writable / Dirty)
    CACHE_NC    = 0b001 << 3, // Önbelleksiz (MMIO için)
    CACHE_WB    = 0b011 << 3, // Write Back (Normal bellek için)
    
    // LoongArch'ta adres 12. bitten başlar, alt 12 bit bayraklardır.
    ADDR_MASK   = 0xFFFFFFFFFFFFF000,
}

/// Basitleştirilmiş Sayfa Tablosu Girişi (PTE)
/// MIPS TLB'yi doldurmak için kullanılan bilgi yapısı.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

/// Yazılımla yönetilen Sayfa Tablosu (RAM'de tutulan veri yapısı)
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; TLB_PTE_COUNT],
}

// -----------------------------------------------------------------------------
// CP0 YAZMAÇLARI (CONTROL PROCESSOR 0)
// -----------------------------------------------------------------------------

// MIPS'te MMU, TLB ve istisna kontrolü CP0 yazmaçları ile yapılır.
// Bazı kritik CP0 yazmaç numaraları (Select 0 varsayımıyla):
const CP0_INDEX: u32    = 0;
const CP0_ENTRY_LO0: u32 = 2; // TLB EntryLo0 yazmacı
const CP0_ENTRY_LO1: u32 = 3; // TLB EntryLo1 yazmacı
const CP0_ENTRY_HI: u32 = 10; // TLB EntryHi yazmacı
const CP0_PAGE_MASK: u32 = 5; // TLB PageMask yazmacı
const CP0_STATUS: u32   = 12; // Durum yazmacı

/// CP0 yazmacını oku.
#[inline(always)]
unsafe fn read_cp0(reg: u32) -> u64 {
    let value: u64;
    // MIPS assembly: 'mfc0 rt, rd'
    // Burada rd = reg, rt = value'nun kaydedildiği yazmaç.
    asm!("mfc0 {0}, {1}", out(reg) value, in(reg) reg, options(nomem, nostack));
    value
}

/// CP0 yazmacına yaz.
#[inline(always)]
unsafe fn write_cp0(reg: u32, value: u64) {
    // MIPS assembly: 'mtc0 rt, rd'
    asm!("mtc0 {0}, {1}", in(reg) value, in(reg) reg, options(nomem, nostack));
}

/// TLB'ye giriş yazar (Index yazmacından belirlenen yere).
#[inline(always)]
unsafe fn tlb_write() {
    // MIPS assembly: 'tlbwr'
    asm!("tlbwr", options(nomem, nostack));
}

/// TLB'yi temizler (tüm girişleri geçersiz kılar).
/// Bu, bir döngü içinde `tlbwi` veya `tlbwr` ile `EntryLo`'yu 0'a ayarlayarak yapılır.
unsafe fn tlb_clear_all() {
    let mut index = 0;
    // MIPS TLB boyutu genellikle 32 veya 64'tür.
    while index < 64 { 
        // Index yazmacına yaz
        write_cp0(CP0_INDEX, index);
        // EntryLo'ları 0'la
        write_cp0(CP0_ENTRY_LO0, 0);
        write_cp0(CP0_ENTRY_LO1, 0);
        // TLB'ye yaz (tlbwi: write indexed)
        asm!("tlbwi", options(nomem, nostack));
        index += 1;
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA VE AKTİVASYON
// -----------------------------------------------------------------------------

/// Sanal adresi fiziksel adrese eşler ve TLB'ye yazar (4KiB sayfa).
///
/// # Parametreler
/// * `tlb_index`: TLB'de yazılacak girişin indeksi.
/// * `virtual_addr`: Sanal adres (Çift sayfa çifti için çift olmalıdır).
/// * `phys_addr_low`: Sanal adrese karşılık gelen fiziksel adres (Çift sayfa çiftinin altı).
/// * `phys_addr_high`: Sanal adrese +4K karşılık gelen fiziksel adres (Çift sayfa çiftinin üstü).
/// * `flags`: Sayfa bayrakları.
pub unsafe fn map_tlb_entry(
    tlb_index: u32,
    virtual_addr: usize,
    phys_addr_low: usize,
    phys_addr_high: usize,
    flags: u64,
) {
    // 1. PageMask (Sayfa Boyutu) ayarla
    // 4K sayfa için PageMask = 0 (12. bit 0, 13. bit 1) -> 0x0
    write_cp0(CP0_PAGE_MASK, 0); 
    
    // 2. EntryHi'yi ayarla (Sanal Adres ve ASID)
    // ASID = 0 (Çekirdek ASID)
    let entry_hi = (virtual_addr as u64) & 0xFFFF_FFFF_FFFF_E000; // Sayfa numarası ve ASID (0)
    write_cp0(CP0_ENTRY_HI, entry_hi);

    // 3. EntryLo0 ve EntryLo1'i ayarla (Fiziksel Adres ve Bayraklar)
    // MIPS TLB'de her giriş 8K'lık bir alanı eşler (iki adet 4K sayfa).
    
    // EntryLo0 (VA: virtual_addr'a eşlenir)
    let entry_lo0 = ((phys_addr_low as u64) >> 6) | flags; // PFN altı 6 bit kesilir
    write_cp0(CP0_ENTRY_LO0, entry_lo0);
    
    // EntryLo1 (VA: virtual_addr + 4K'ya eşlenir)
    let entry_lo1 = ((phys_addr_high as u64) >> 6) | flags;
    write_cp0(CP0_ENTRY_LO1, entry_lo1);

    // 4. Index'i ayarla ve TLB'ye yaz
    write_cp0(CP0_INDEX, tlb_index);
    tlb_write();
    
    io::sync(); // Senkronizasyon bariyeri
}


/// Sayfalama (TLB) mekanizmasını etkinleştirir.
pub unsafe fn enable_paging() {
    serial_println!("[MIPS64] İlk TLB Girişleri Dolduruluyor...");
    
    // TLB'yi temizle
    tlb_clear_all();
    
    // Varsayımsal Birebir Eşleme (İlk 16MB)
    let identity_mapping_size = 16 * 1024 * 1024; // 16 MB
    let flags = EntryLoFlags::VALID as u64 
              | EntryLoFlags::DIRTY as u64 
              | EntryLoFlags::GLOBAL as u64 
              | EntryLoFlags::CACHE_WB as u64; 

    // Her TLB girişi 8KB eşlediği için 8KB adımlarla döngü yapılır
    let mut tlb_idx = 0;
    for addr in (0..identity_mapping_size).step_by(PAGE_SIZE * 2) {
        if tlb_idx >= 64 { break; } // TLB doldu
        
        let phys_low = addr;
        let phys_high = addr + PAGE_SIZE;

        map_tlb_entry(tlb_idx, addr, phys_low, phys_high, flags);
        tlb_idx += 1;
    }
    
    serial_println!("[MIPS64] {} adet 8KB TLB girişi (16MB) eşlendi.", tlb_idx);
    
    // MIPS'te sayfalama, CP0 Status yazmacındaki 'UM', 'ERL', 'EXL', ve 'BEV' 
    // gibi bitlerin ayarlanmasıyla dolaylı olarak kontrol edilir.
    // ERL=0 ve EXL=0 olduğunda sanal adresleme (TLB) kullanılır.
    
    let mut status = read_cp0(CP0_STATUS);
    // ERL (Error Level) ve EXL (Exception Level) bitlerinin 0 olduğundan emin olunur.
    status &= !((1 << 1) | (1 << 2)); // ERL ve EXL'yi temizle
    
    // Küresel kesmeleri etkinleştir
    status |= 1 << 0; // IE (Interrupt Enable) bitini ayarla
    
    write_cp0(CP0_STATUS, status);
    
    io::sync(); // Senkronizasyon bariyeri
    
    serial_println!("[MIPS64] Sanal Adresleme (TLB) etkinleştirildi.");
}


/// Sayfalama sonrası çekirdek başlatma işlevi.
/// `main.rs` içinden çağrılmalıdır.
pub fn init_mmu() {
    unsafe {
        enable_paging();
    }
}