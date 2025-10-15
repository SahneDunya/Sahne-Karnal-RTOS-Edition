#![allow(dead_code)]
#![allow(non_snake_case)]

use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;
use crate::serial_println;

// -----------------------------------------------------------------------------
// RISC-V MMIO ADRESLERİ (QEMU 'virt' varsayımı)
// -----------------------------------------------------------------------------

// Bu adresler DTB'den (Device Tree) okunmalıdır, ancak temsili adresler kullanıyoruz.
const CLINT_BASE: usize = 0x200_0000;
const PLIC_BASE: usize  = 0xC00_0000;

// CLINT Yazmaç Ofsetleri
const MTIMECMP: usize = 0x4000;  // Zamanlayıcı Karşılaştırma Yazmacı (64-bit)
const MTIME: usize    = 0xBFF8;  // Geçen Süre Yazmacı (64-bit)
const MSIP: usize     = 0x0;     // Yazılım Kesmesi Yazmacı (32-bit)

// PLIC Yazmaç Ofsetleri
const PLIC_PRIORITY_BASE: usize = 0x000004; // Kesme Öncelikleri (4 bayt/IRQ)
const PLIC_ENABLE_BASE: usize   = 0x002000; // S-Mode Etkinleştirme Yazmaçları
const PLIC_THRESHOLD: usize     = 0x200000; // S-Mode Eşik Yazmacı (Kesme Önceliği)
const PLIC_CLAIM: usize         = 0x200004; // S-Mode Talep/EOI Yazmacı (Claim/EOI)

// -----------------------------------------------------------------------------
// MMIO VE PIC/CLINT KONTROLÜ
// -----------------------------------------------------------------------------

/// MMIO adresinden 32 bitlik veri okur.
#[inline(always)]
unsafe fn mmio_read_u32(addr: usize) -> u32 {
    read_volatile(addr as *const u32)
}

/// MMIO adresine 32 bitlik veri yazar.
#[inline(always)]
unsafe fn mmio_write_u32(addr: usize, value: u32) {
    write_volatile(addr as *mut u32, value)
}

/// MMIO adresinden 64 bitlik veri okur.
#[inline(always)]
unsafe fn mmio_read_u64(addr: usize) -> u64 {
    read_volatile(addr as *const u64)
}

/// MMIO adresine 64 bitlik veri yazar.
#[inline(always)]
unsafe fn mmio_write_u64(addr: usize, value: u64) {
    write_volatile(addr as *mut u64, value)
}

// -----------------------------------------------------------------------------
// 1. PLIC (Platform-Level Interrupt Controller) YÖNETİMİ
// -----------------------------------------------------------------------------

pub struct Plic;

impl Plic {
    // Tüm PLIC kesmelerini maskele ve öncelikleri sıfırla.
    pub unsafe fn init() {
        // Tüm S-Mode kesmelerini devre dışı bırak.
        mmio_write_u32(PLIC_BASE + PLIC_ENABLE_BASE, 0); 
        
        // Eşik (Threshold) değerini 0'a ayarla: 
        // Önceliği 0'dan büyük tüm kesmeleri kabul et.
        mmio_write_u32(PLIC_BASE + PLIC_THRESHOLD, 0); 
        
        // Tüm kesme önceliklerini 1'e ayarla (1 en düşük önceliktir).
        // 1. kesmeden başlar (IRQ 0 rezerve edilmiştir).
        for irq in 1..256 {
            mmio_write_u32(PLIC_BASE + PLIC_PRIORITY_BASE + (irq * 4), 1);
        }
        
        serial_println!("[RV64I] PLIC Başlatıldı.");
    }

    /// Harici IRQ'yu etkinleştir (S-Mode).
    pub unsafe fn enable_irq(irq_id: u32) {
        let index = irq_id / 32;
        let bit = irq_id % 32;
        let addr = PLIC_BASE + PLIC_ENABLE_BASE + (index as usize) * 4;
        
        let mut enable = mmio_read_u32(addr);
        enable |= 1 << bit;
        mmio_write_u32(addr, enable);
    }
    
    /// Harici IRQ'yu devre dışı bırak (S-Mode).
    pub unsafe fn disable_irq(irq_id: u32) {
        let index = irq_id / 32;
        let bit = irq_id % 32;
        let addr = PLIC_BASE + PLIC_ENABLE_BASE + (index as usize) * 4;
        
        let mut enable = mmio_read_u32(addr);
        enable &= !(1 << bit);
        mmio_write_u32(addr, enable);
    }

    /// İşlenecek bekleyen kesmenin ID'sini alır (Acknowledge).
    pub unsafe fn claim_irq() -> u32 {
        mmio_read_u32(PLIC_BASE + PLIC_CLAIM)
    }

    /// Kesmenin işlenmesi bittiğini PLIC'e bildirir (EOI).
    pub unsafe fn complete_irq(irq_id: u32) {
        mmio_write_u32(PLIC_BASE + PLIC_CLAIM, irq_id);
    }
}

// -----------------------------------------------------------------------------
// 2. CLINT (Core Local Interrupt Controller) YÖNETİMİ
// -----------------------------------------------------------------------------

pub struct Clint;

impl Clint {
    /// Yazılım kesmesini tetikler (S-Mode'da Yazılım Kesmesi).
    pub unsafe fn trigger_software_interrupt() {
        // MSIP'e yaz (S-mode için MSIP'in alt 32 biti)
        mmio_write_u32(CLINT_BASE + MSIP, 1);
    }

    /// Yazılım kesmesini temizler.
    pub unsafe fn clear_software_interrupt() {
        mmio_write_u32(CLINT_BASE + MSIP, 0);
    }

    /// Zamanlayıcı kesmesini bir sonraki ana ayarla.
    pub unsafe fn set_next_timer_interrupt(interval: u64) {
        let current_time = mmio_read_u64(CLINT_BASE + MTIME);
        // Zamanlayıcıyı mevcut zamana interval ekleyerek ayarla
        mmio_write_u64(CLINT_BASE + MTIMECMP, current_time + interval);
    }
}

// -----------------------------------------------------------------------------
// 3. GENEL KESME İŞLEME MANTIĞI
// -----------------------------------------------------------------------------

/// Gelen Harici Kesmeleri (PLIC) İşleme.
pub fn handle_external_interrupts() {
    unsafe {
        // PLIC'ten bekleyen IRQ ID'sini al (Claim)
        let irq_id = Plic::claim_irq();

        if irq_id > 0 && irq_id < 1024 { // Geçerli bir IRQ olduğunu varsayalım
            // serial_println!("Harici IRQ {} geldi.", irq_id);
            
            // 1. IRQ'yu işle (Uygun sürücüyü çağır)
            // driver::handle_irq(irq_id);
            
            // 2. EOI gönder
            Plic::complete_irq(irq_id);
        }
    }
}

/// Gelen Zamanlayıcı Kesmelerini (CLINT) İşleme.
pub fn handle_timer_interrupt() {
    // serial_println!("Zamanlayıcı kesmesi!");
    
    // 1. Bir sonraki kesme anını ayarla (Örn: 1 saniye sonra)
    unsafe {
        // 1 saniye = 10.000.000 döngü (Temsili, frekans 10MHz varsayımıyla)
        Clint::set_next_timer_interrupt(10_000_000); 
    }
    
    // 2. İşletim sistemi zamanlayıcı/görev değiştirici (scheduler) mantığını çağır.
}

/// Tüm kesme kontrolcülerini başlatır.
pub fn init_interrupts() {
    unsafe {
        // PLIC'i başlat ve tüm harici kesmeleri devre dışı bırak
        Plic::init();
        
        // Zamanlayıcıyı ayarla (İlk kesmeyi hemen ardından ayarla)
        Clint::set_next_timer_interrupt(10_000_000); 
        
        // M-mode yazılımlarının kullanabileceği yazılımları temizle (varsa)
        Clint::clear_software_interrupt();
    }
    
    serial_println!("[RV64I] CLINT/PLIC Başlatıldı.");
}