#![allow(dead_code)]

use core::marker::PhantomData;
use core::slice;
use crate::platformgeneric::KernelError;

// Bu, FDT ayrıştırma işlemlerini simüle eden temel yapılardır.
// Gerçek projede 'device-tree' gibi bir crate'in kullanılması gerekir.

/// Aygıt Ağacı Kaynağı (Device Tree Source) adresi ve boyutu.
#[derive(Debug, Clone, Copy)]
pub struct DtbInfo {
    /// FDT'nin fiziksel bellek adresi.
    pub physical_address: usize,
    /// Ayrıştırılmış yapıdan çıkarılan temel donanım yapılandırması.
    pub config: Option<HardwareConfig>,
}

/// Aygıt Ağacından okunan temel donanım yapılandırmaları.
#[derive(Debug, Clone, Copy)]
pub struct HardwareConfig {
    /// Seri Port (UART) MMIO adresi
    pub console_addr: usize,
    /// Bellek Başlangıç Adresi
    pub ram_start: usize,
    /// Toplam Bellek Boyutu (bayt)
    pub ram_size: usize,
    /// Platform Level Interrupt Controller (PLIC) adresi
    pub plic_addr: usize,
    /// Core Local Interruptor (CLINT) adresi
    pub clint_addr: usize,
}

/// Aygıt Ağacını ayrıştırmaktan sorumlu statik yapı.
pub struct DtbParser {
    _phantom: PhantomData<()>,
}

// DTB'nin zaten ayrıştırılıp ayrıştırılmadığını izler
static mut DTB_INFO: DtbInfo = DtbInfo {
    physical_address: 0,
    config: None,
};

impl DtbParser {
    /// Önyükleyici tarafından çekirdeğe iletilen FDT adresini kaydeder.
    ///
    /// RISC-V'de, FDT adresi genellikle `a1` kaydında iletilir.
    pub fn set_dtb_address(addr: usize) {
        if addr != 0 {
            unsafe {
                DTB_INFO.physical_address = addr;
            }
        }
    }

    /// FDT Blob'unu ayrıştırır ve donanım yapılandırmasını çıkarır.
    ///
    /// # Parametreler
    /// * `dtb_addr`: FDT'nin bellekteki adresi.
    pub fn parse_dtb(dtb_addr: usize) -> Result<HardwareConfig, KernelError> {
        if dtb_addr == 0 {
            return Err(KernelError::DtbNotFound);
        }

        // --- Gerçek FDT ayrıştırma (device-tree crate simülasyonu) ---
        // RISC-V'de tüm donanım (UART, PLIC, CLINT) adresleri DTB'den alınır.

        // Simülasyon: RISC-V 64 virt/SiFive platformunda tipik donanım adresleri
        let config = HardwareConfig {
            // Seri Port (UART) adresi (Örn: 16550 Uyumlu UART, QEMU virt)
            console_addr: 0x1000_0000, 
            
            // Bellek bilgisi (genellikle 'memory' düğümünden alınır, QEMU virt)
            ram_start: 0x8000_0000, 
            ram_size: 128 * 1024 * 1024, // 128MB
            
            // Kesme Kontrolcüleri (QEMU virt varsayılanları)
            plic_addr: 0x0C00_0000, // Platform Level Interrupt Controller
            clint_addr: 0x0200_0000, // Core Local Interruptor
        };
        
        unsafe {
            DTB_INFO.config = Some(config);
        }

        Ok(config)
    }

    /// Ayrıştırılmış yapılandırmayı döndürür.
    pub fn get_config() -> Result<&'static HardwareConfig, KernelError> {
        unsafe {
            DTB_INFO.config.as_ref().ok_or(KernelError::ConfigurationNotParsed)
        }
    }
}

// -----------------------------------------------------------------------------
// Çekirdek Loglama/Hata Ayıklama için Basit FDT Dökümü
// -----------------------------------------------------------------------------

/// Sınırlı miktarda DTB başlığını çıktılamak için kullanılır.
pub fn dump_dtb_header() {
    let dtb_addr = unsafe { DTB_INFO.physical_address };
    if dtb_addr == 0 {
        // serial_println! macro'sunun arch/rv64i/console.rs'te tanımlandığını varsayıyoruz.
        serial_println!("[DTB-RV64I] DTB adresi bilinmiyor (0x0).");
        return;
    }
    
    // Güvenlik: Adresin geçerli bir bellek aralığında olduğunu varsayıyoruz.
    let header_slice = unsafe { 
        slice::from_raw_parts(dtb_addr as *const u32, 4) 
    };

    // Rust'ta Big Endian (dtb formatı) okuması için u32::from_be kullanılır.
    let magic = u32::from_be(header_slice[0]);

    serial_println!("[DTB-RV64I] DTB Adresi: {:#x}", dtb_addr);
    serial_println!("[DTB-RV64I] Magic (Beklenen 0xd00dfeed): {:#x}", magic);
    
    // Ayrıştırılmış yapılandırma varsa onu da bas
    if let Ok(config) = DtbParser::get_config() {
        serial_println!("[DTB-RV64I] -> Konsol Adresi: {:#x}", config.console_addr);
        serial_println!("[DTB-RV64I] -> PLIC Adresi: {:#x}", config.plic_addr);
        serial_println!("[DTB-RV64I] -> CLINT Adresi: {:#x}", config.clint_addr);
        serial_println!("[DTB-RV64I] -> RAM Boyutu: {} MB", config.ram_size / 1024 / 1024);
    } else {
        serial_println!("[DTB-RV64I] -> Yapılandırma henüz ayrıştırılmadı.");
    }
}