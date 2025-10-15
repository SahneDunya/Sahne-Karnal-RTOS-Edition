#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr::NonNull;
use crate::serial_println;
use super::io; // Bariyerler için io modülünü kullanacağız

// -----------------------------------------------------------------------------
// SAYFALAMA SABİTLERİ VE TİPLERİ (4K Sayfa, Sv39 Varsayımı)
// -----------------------------------------------------------------------------

/// Sayfa boyutu: 4 KiB
pub const PAGE_SIZE: usize = 4096;
pub const TABLE_ENTRY_COUNT: usize = 512; // 9-bit indeksleme için

// Sayfa Tablosu Girişi (PTE) bayrakları (RISC-V standardı)
#[repr(u64)]
pub enum PageFlags {
    // Erişim Bayrakları
    VALID       = 1 << 0,  // Giriş geçerli (veya sonraki seviyeye işaret ediyor)
    READ        = 1 << 1,  // Okunabilir
    WRITE       = 1 << 2,  // Yazılabilir
    EXEC        = 1 << 3,  // Yürütülebilir
    USER        = 1 << 4,  // Kullanıcı seviyesi erişebilir
    GLOBAL      = 1 << 5,  // Global
    ACCESSED    = 1 << 6,  // Erişildi
    DIRTY       = 1 << 7,  // Yazıldı

    // Fiziksel Sayfa Numarası (PPN) 10. bitten başlar
    PPN_MASK    = 0x003F_FFFF_FFFF_FC00,
}

/// Sayfa Tablosu Girişi (PTE)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

/// Sv39 Sayfa Tablosu (512 giriş, 4096 bayt)
/// Seviye 1, 2 ve 3 için kullanılır.
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; TABLE_ENTRY_COUNT],
}

// -----------------------------------------------------------------------------
// PTE UYGULAMASI
// -----------------------------------------------------------------------------

impl PageTableEntry {
    /// Girişi sıfırlar.
    pub const fn zero() -> Self {
        PageTableEntry(0)
    }

    /// Girişin geçerli olup olmadığını kontrol eder (V bit).
    pub fn is_valid(&self) -> bool {
        (self.0 & (PageFlags::VALID as u64)) != 0
    }

    /// Yeni bir tablo girişi oluşturur (L1 veya L2).
    pub fn new_table(addr: usize) -> Self {
        // Tablo girişleri için sadece VALID bayrağı ayarlanır. (R/W/X = 0)
        let flags = PageFlags::VALID as u64; 
        
        // PPN (Physical Page Number) = Fiziksel Adres / 4096. PPN 10. bitten başlar.
        let ppn = (addr / PAGE_SIZE) as u64;

        PageTableEntry((ppn << 10) | flags)
    }

    /// Yeni bir sayfa girişi oluşturur (L3).
    pub fn new_page(addr: usize, flags: u64) -> Self {
        // Sayfa girişleri için R/W/X bayrakları ve VALID ayarlanır.
        let ppn = (addr / PAGE_SIZE) as u64;
        
        PageTableEntry((ppn << 10) | flags | PageFlags::VALID as u64)
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK SAYFALAMA YÖNETİMİ
// -----------------------------------------------------------------------------

/// Sayfa tablosu hiyerarşisinde verilen sanal adrese karşılık gelen dizinleri döndürür.
/// Sv39 (39-bit VAddr, 3 seviye) varsayımı.
fn get_indices(virtual_addr: usize) -> (usize, usize, usize) {
    // L1 indeksi: [38:30] bitleri
    let l1 = (virtual_addr >> 30) & 0x1FF;
    // L2 indeksi: [29:21] bitleri
    let l2 = (virtual_addr >> 21) & 0x1FF;
    // L3 indeksi: [20:12] bitleri
    let l3 = (virtual_addr >> 12) & 0x1FF;
    
    (l1, l2, l3) 
}

/// Yeni bir sayfa tablosu tahsis etmeyi simüle eder.
fn alloc_page_table() -> NonNull<PageTable> {
    // Mock Tahsis
    static mut PAGE_TABLE_MOCK: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    
    let ptr = unsafe { PAGE_TABLE_MOCK.as_mut_ptr() as *mut PageTable };
    let table = unsafe { &mut *ptr };
    table.entries.iter_mut().for_each(|e| *e = PageTableEntry::zero());
    
    unsafe { NonNull::new_unchecked(ptr) }
}


/// Sanal adresi fiziksel adrese eşler (4KiB sayfa).
pub unsafe fn map_page(
    root_table_addr: usize, // L1 tablosunun fiziksel adresi
    virtual_addr: usize,
    physical_addr: usize,
    flags: u64,
) {
    let (l1i, l2i, l3i) = get_indices(virtual_addr);
    let l1_table = &mut *(root_table_addr as *mut PageTable);
    
    // 1. L2'yi al veya oluştur
    let l2_entry = l1_table.entries.get_mut(l1i).expect("L1 Index Hata");
    let l2_addr = if l2_entry.is_valid() {
        (l2_entry.0 & PageFlags::PPN_MASK as u64) << 2 // PPN -> Fiziksel Adres
    } else {
        let new_l2 = alloc_page_table();
        let new_addr = new_l2.as_ptr() as usize;
        *l2_entry = PageTableEntry::new_table(new_addr);
        new_addr as u64
    } as usize;
    
    let l2_table = &mut *(l2_addr as *mut PageTable);
    
    // 2. L3'ü al veya oluştur
    let l3_entry = l2_table.entries.get_mut(l2i).expect("L2 Index Hata");
    let l3_addr = if l3_entry.is_valid() {
        (l3_entry.0 & PageFlags::PPN_MASK as u64) << 2
    } else {
        let new_l3 = alloc_page_table();
        let new_addr = new_l3.as_ptr() as usize;
        *l3_entry = PageTableEntry::new_table(new_addr);
        new_addr as u64
    } as usize;

    let l3_table = &mut *(l3_addr as *mut PageTable);
    
    // 3. L4 Girişini ayarla (Son eşleme, 4K Sayfa)
    let final_entry = l3_table.entries.get_mut(l3i).expect("L3 Index Hata");
    *final_entry = PageTableEntry::new_page(physical_addr, flags);
    
    // Sayfa tablosu güncellendi, TLB temizliği (fence.i) gereklidir.
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA VE AKTİVASYON
// -----------------------------------------------------------------------------

// CSR'lara erişim için yardımcı fonksiyonlar
#[inline(always)]
unsafe fn read_csr(csr_num: u32) -> u64 {
    let value: u64;
    // RISC-V assembly: 'csrr t0, csr_num' (Sistem yazmaçlarını okuma)
    asm!("csrr {0}, {1}", out(reg) value, in(reg) csr_num);
    value
}

#[inline(always)]
unsafe fn write_csr(csr_num: u32, value: u64) {
    // RISC-V assembly: 'csrw csr_num, t0' (Sistem yazmaçlarına yazma)
    asm!("csrw {0}, {1}", in(reg) csr_num, in(reg) value);
}

/// Sayfalama için yeni L1 tablosunu hazırlar ve sanal adresleri eşler.
pub fn setup_initial_paging() -> usize {
    serial_println!("[RV64I] Sv39 Sayfalama Hazırlanıyor...");
    
    // Statik olarak hizalanmış bir bellek bloğunu L1 (Kök) Tablo olarak kullan.
    static mut L1_TABLE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    
    let l1_addr = unsafe { L1_TABLE.as_mut_ptr() as usize };
    
    // Sayfa Tablosunu sıfırla
    let l1_table = unsafe { &mut *(l1_addr as *mut PageTable) };
    l1_table.entries.iter_mut().for_each(|e| *e = PageTableEntry::zero());

    // Çekirdek eşlemesi için bayraklar (RWX, Global, Dirty, Accessed)
    let kernel_flags = PageFlags::READ as u64 
                     | PageFlags::WRITE as u64 
                     | PageFlags::EXEC as u64 
                     | PageFlags::GLOBAL as u64
                     | PageFlags::ACCESSED as u64
                     | PageFlags::DIRTY as u64; 

    // Örnek eşleme: İlk 16MB'ı birebir (identity) eşle.
    let identity_mapping_size = 16 * 1024 * 1024; // 16 MB

    for addr in (0..identity_mapping_size).step_by(PAGE_SIZE) {
        unsafe {
            // Sanal ve fiziksel adresler aynı kabul edilir (birebir eşleme)
            map_page(l1_addr, addr, addr, kernel_flags);
        }
    }

    serial_println!("[RV64I] İlk 16MB birebir eşlendi (Identity Mapped).");
    l1_addr
}

/// Sayfalama mekanizmasını etkinleştirir.
///
/// # Parametreler
/// * `l1_phys_addr`: Yeni L1 tablosunun fiziksel adresi.
pub unsafe fn enable_paging(l1_phys_addr: usize) {
    // 1. satp (Supervisor Address Translation and Protection) yazmacını ayarla
    // satp = (MODE << 60) | (ASID << 44) | PPN
    // Sv39 için MODE = 8 (0b1000)
    const SATP_MODE_SV39: u64 = 8;
    const CSR_SATP: u32 = 0x180; // satp CSR numarası
    
    let ppn = (l1_phys_addr / PAGE_SIZE) as u64;
    
    let satp_val = (SATP_MODE_SV39 << 60) | ppn;
    
    write_csr(CSR_SATP, satp_val);

    // 2. Talimat boru hattını temizle (fence.i)
    // satp yazıldıktan sonra MMU hemen etkinleşir, bu yüzden I-Sync gereklidir.
    io::fence_i(); 

    serial_println!("[RV64I] Sv39 Sayfalama etkinleştirildi. Kök PPN: {:#x}", ppn);
}


/// Sayfalama sonrası çekirdek başlatma işlevi.
/// `main.rs` içinden çağrılmalıdır.
pub fn init_mmu() {
    // İlk sayfa tablosunu hazırla
    let l1_addr = setup_initial_paging();
    
    // Sayfalamayı etkinleştir
    unsafe {
        // Not: l1_addr'ın fiziksel adresi olduğundan emin olun.
        enable_paging(l1_addr);
    }
}