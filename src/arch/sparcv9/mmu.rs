#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr::NonNull;
use crate::serial_println;
use super::io; // Bariyerler için io modülünü kullanacağız

// -----------------------------------------------------------------------------
// SAYFALAMA SABİTLERİ VE TİPLERİ (8K Sayfa Varsayımı)
// -----------------------------------------------------------------------------

/// Sayfa boyutu: 8 KiB (UltraSPARC'ta yaygın olan sayfa boyutu)
pub const PAGE_SIZE: usize = 8192;
pub const TABLE_ENTRY_COUNT: usize = 512; // 9-bit indeksleme için

// Sayfa Tablosu Girişi (PTE) bayrakları (UltraSPARC standardına göre temsili)
#[repr(u64)]
pub enum PageFlags {
    // Tip ve Durum Bayrakları
    TYPE_MASK   = 0x3,     // En düşük 2 bit
    INVALID     = 0b00,    // Geçersiz giriş (0)
    TABLE       = 0b01,    // Sonraki seviye tablosu (1)
    PAGE_8K     = 0b10,    // 8K Sayfa (2)
    
    // Erişim ve Önbellek Bayrakları (High bits)
    ACCESSED    = 1 << 5,  // Erişildi
    MODIFIED    = 1 << 6,  // Yazıldı (Dirty)
    CACHEABLE   = 1 << 7,  // Önbellek etkin
    PRIVILEGED  = 1 << 8,  // Sadece süpervizör modu erişebilir
    
    // Yürütme/Yazma İzinleri (Perms)
    WRITE_ENA   = 1 << 10, // Yazma izni
    EXEC_ENA    = 1 << 11, // Yürütme izni
    
    // Fiziksel Sayfa Numarası (PFN) 13. bitten başlar
    PFN_MASK    = 0xFFFF_FFFF_FFFF_E000,
}

/// Sayfa Tablosu Girişi (PTE)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

/// 3 seviyeli Sayfa Tablosu (512 giriş, 4096 bayt)
/// L1, L2, L3 için kullanılır (4K/8K/64K/512K sayfa boyutları desteklenir).
#[repr(align(8192))] // 8K Hizalama
pub struct PageTable {
    entries: [PageTableEntry; TABLE_ENTRY_COUNT],
}

// -----------------------------------------------------------------------------
// MMU KONTROL YAZMAÇLARI
// -----------------------------------------------------------------------------

// SPARC'ta MMU kontrol yazmaçlarına erişim özel ASI (Address Space Identifier) 
// ile Load/Store talimatları kullanılarak yapılır.
const ASI_MMU_CONTROL: u8 = 0x40; // Temsili MMU Control ASI

/// Özel bir MMU yazmacını okur (ASI kullanarak).
#[inline(always)]
unsafe fn read_mmu_reg(offset: usize) -> u64 {
    let value: u64;
    // SPARC assembly: 'ldxa [rs1 + offset] asi, rd'
    asm!("ldxa [%g0, {offset}] {asi}, {value}", 
         offset = in(reg) offset, 
         asi = const ASI_MMU_CONTROL, 
         value = out(reg) value);
    value
}

/// Özel bir MMU yazmacına yazar (ASI kullanarak).
#[inline(always)]
unsafe fn write_mmu_reg(offset: usize, value: u64) {
    // SPARC assembly: 'stxa rs2, [rs1 + offset] asi'
    asm!("stxa {value}, [%g0, {offset}] {asi}", 
         value = in(reg) value, 
         offset = in(reg) offset, 
         asi = const ASI_MMU_CONTROL);
    io::membar_all(); // MMU yazma işleminden sonra tam bariyer gereklidir.
}

// MMU Kontrol Yazmaçlarının Ofsetleri (Temsili)
const MMU_CR_OFFSET: usize = 0x000; // MMU Control Register
const MMU_TSB_BASE_OFFSET: usize = 0x010; // TSB Base Register

// -----------------------------------------------------------------------------
// ÇEKİRDEK SAYFALAMA YÖNETİMİ
// -----------------------------------------------------------------------------

/// Yeni bir sayfa tablosu tahsis etmeyi simüle eder.
fn alloc_page_table() -> NonNull<PageTable> {
    // Mock Tahsis
    static mut PAGE_TABLE_MOCK: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    
    let ptr = unsafe { PAGE_TABLE_MOCK.as_mut_ptr() as *mut PageTable };
    let table = unsafe { &mut *ptr };
    table.entries.iter_mut().for_each(|e| *e = PageTableEntry(0));
    
    unsafe { NonNull::new_unchecked(ptr) }
}


/// Sayfa tablosu hiyerarşisinde verilen sanal adrese karşılık gelen dizinleri döndürür.
/// 3 seviyeli 8K sayfalama varsayımı.
fn get_indices(virtual_addr: usize) -> (usize, usize, usize) {
    // 3 seviye: 39-bit VAddr, 3 x 9 bit indeks (VADDR[40:32], VADDR[31:23], VADDR[22:14])
    // 8K sayfa boyutu (13 bit ofset)
    
    // L1 indeksi: [31:39] bitleri
    let l1 = (virtual_addr >> 30) & 0x1FF;
    // L2 indeksi: [22:30] bitleri
    let l2 = (virtual_addr >> 21) & 0x1FF;
    // L3 indeksi: [13:21] bitleri
    let l3 = (virtual_addr >> 12) & 0x1FF; // Yanlış: 13 bit ofset olduğu için 13'ten başlamalı
    
    // Düzeltilmiş (8K sayfa için 13 bit ofset, yani 13. bitten başlar):
    let l1 = (virtual_addr >> 39) & 0x1FF; // VADDR[47:39]
    let l2 = (virtual_addr >> 30) & 0x1FF; // VADDR[38:30]
    let l3 = (virtual_addr >> 21) & 0x1FF; // VADDR[29:21] - Son seviye (8K/4K/...)
    
    (l1, l2, l3) 
}

/// Sanal adresi fiziksel adrese eşler.
pub unsafe fn map_page(
    root_table_addr: usize, // L1 tablosunun fiziksel adresi
    virtual_addr: usize,
    physical_addr: usize,
    flags: u64,
) {
    let (l1i, l2i, l3i) = get_indices(virtual_addr);
    let l1_table = &mut *(root_table_addr as *mut PageTable);
    
    // L1 -> L2 Tablosu
    let l2_entry = l1_table.entries.get_mut(l1i).expect("L1 Index Hata");
    let l2_addr = if (l2_entry.0 & PageFlags::TYPE_MASK as u64) == PageFlags::TABLE as u64 {
        l2_entry.0 & PageFlags::PFN_MASK as u64
    } else {
        let new_l2 = alloc_page_table();
        let new_addr = new_l2.as_ptr() as usize;
        *l2_entry = PageTableEntry((new_addr as u64) | PageFlags::TABLE as u64);
        new_addr as u64
    } as usize;
    
    let l2_table = &mut *(l2_addr as *mut PageTable);
    
    // L2 -> L3 Tablosu
    let l3_entry = l2_table.entries.get_mut(l2i).expect("L2 Index Hata");
    let l3_addr = if (l3_entry.0 & PageFlags::TYPE_MASK as u64) == PageFlags::TABLE as u64 {
        l3_entry.0 & PageFlags::PFN_MASK as u64
    } else {
        let new_l3 = alloc_page_table();
        let new_addr = new_l3.as_ptr() as usize;
        *l3_entry = PageTableEntry((new_addr as u64) | PageFlags::TABLE as u64);
        new_addr as u64
    } as usize;

    let l3_table = &mut *(l3_addr as *mut PageTable);
    
    // Son eşleme: L3 Tablosunda 8K Sayfa Girişi
    let final_entry = l3_table.entries.get_mut(l3i).expect("L3 Index Hata");
    
    // Bayrakları ve PFN'i birleştir
    let pfn = (physical_addr as u64) & PageFlags::PFN_MASK as u64;
    *final_entry = PageTableEntry(pfn | flags | PageFlags::PAGE_8K as u64);
    
    // TSB'yi temizle (işlemciye TLB/TSB'nin güncellenmesi gerektiğini bildir)
    io::membar_all();
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA VE AKTİVASYON
// -----------------------------------------------------------------------------

/// TSB (Translation Storage Buffer) temizler (Soft TLB).
/// TSB'ye yazma ile geçersiz kılma yapılır.
unsafe fn tsb_invalidate_all() {
    // TSB'yi temizleme talimatı (Örn: I-MMU'dan I-TSB'yi temizle)
    // Bu, UltraSPARC'ta genellikle özel bir MMU yazmacına yazarak yapılır.
    
    // MMU CR'de TSB temizleme biti ayarlanır (Temsili)
    let cr = read_mmu_reg(MMU_CR_OFFSET);
    // Temsili TSB temizleme biti (UltraSPARC'ta bu bit genellikle yoktur, 
    // bunun yerine TSB Base'i sıfırlamak veya özel bir flush talimatı kullanılır)
    
    // Basitleştirme: Tüm TSB'yi temizleyen özel bir talimat çağrısı varsayalım.
    // %asi 0x14 ile 'ldd' talimatı kullanılarak TSB'yi temizlemek yaygındır.
    asm!("flushw", options(nomem, nostack)); // Temsili bir komut
    io::membar_all();
}


/// Sayfalama mekanizmasını (TSB tabanlı MMU) etkinleştirir.
pub unsafe fn enable_paging(l1_phys_addr: usize) {
    serial_println!("[SPARC V9] MMU (TSB) Hazırlanıyor...");

    // 1. TSB'yi ayarla (Translation Storage Buffer Base Register)
    // TSB Base yazmacı, TSB'nin başlangıç adresini, boyutunu ve MMU'nun bayraklarını içerir.
    const TSB_SIZE_256K: u64 = 0x2; // Temsili 256K TSB boyutu bayrağı
    
    // TSB Base = (TSB Fiziksel Adresi & ~0x1FFF) | TSB Boyutu Bayrakları
    let tsb_base_val = (l1_phys_addr as u64) | TSB_SIZE_256K; // L1 Adresi TSB Base olarak kullanılıyor.
    
    // TSB Base Register'a yaz
    write_mmu_reg(MMU_TSB_BASE_OFFSET, tsb_base_val);

    // 2. TSB'yi temizle
    tsb_invalidate_all();

    // 3. MMU Control Register'ı ayarla (MMU'yu etkinleştir)
    let mut cr = read_mmu_reg(MMU_CR_OFFSET);
    
    // MMU_E (Enable) bitini ayarla (Genellikle 0. bit)
    const MMU_E: u64 = 1 << 0; 
    cr |= MMU_E;

    // MMU Control Register'a yaz
    write_mmu_reg(MMU_CR_OFFSET, cr);

    serial_println!("[SPARC V9] MMU etkinleştirildi. TSB Kök: {:#x}", l1_phys_addr);
}


/// Sayfalama sonrası çekirdek başlatma işlevi.
/// `main.rs` içinden çağrılmalıdır.
pub fn init_mmu() {
    serial_println!("[SPARC V9] MMU Başlatılıyor...");

    // İlk sayfa tablosunu hazırla (Aynı zamanda TSB olarak kullanılacak)
    static mut L1_TABLE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    let l1_addr = unsafe { L1_TABLE.as_mut_ptr() as usize };
    
    // Basitleştirilmiş Birebir Eşleme (16MB)
    let mapping_size = 16 * 1024 * 1024;
    let flags = PageFlags::PRIVILEGED as u64 
              | PageFlags::WRITE_ENA as u64 
              | PageFlags::EXEC_ENA as u64 
              | PageFlags::CACHEABLE as u64
              | PageFlags::ACCESSED as u64
              | PageFlags::MODIFIED as u64;

    for addr in (0..mapping_size).step_by(PAGE_SIZE) {
        unsafe {
            // Sanal ve fiziksel adresler aynı
            map_page(l1_addr, addr, addr, flags);
        }
    }
    serial_println!("[SPARC V9] İlk 16MB birebir eşlendi.");

    // Sayfalamayı etkinleştir
    unsafe {
        // Not: l1_addr'ın fiziksel adresi olduğundan emin olun.
        enable_paging(l1_addr);
    }
}