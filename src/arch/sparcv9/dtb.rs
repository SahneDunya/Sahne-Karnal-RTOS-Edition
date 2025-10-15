#![allow(dead_code)]

use core::marker::PhantomData;
use core::slice;
use crate::platformgeneric::KernelError;

// Bu, FDT ayrıştırma işlemlerini simüle eden temel yapılardır.
// Gerçek projede 'device-tree' veya Open Firmware/PROM ayrıştırma mantığı kullanılmalıdır.

/// Aygıt Ağacı Kaynağı (Device Tree Source) adresi ve boyutu.
#[derive(Debug, Clone, Copy)]
pub struct DtbInfo {
    /// FDT'nin fiziksel bellek adresi (OpenPROM tarafından iletilir).
    pub physical_address: usize,
    /// Ayrıştırılmış yapıdan çıkarılan temel donanım yapılandırması.
    pub config: Option<HardwareConfig>,
}

/// Aygıt Ağacından okunan temel donanım yapılandırmaları.
#[derive(Debug, Clone, Copy)]
pub struct HardwareConfig {
    /// Seri Port (UART) MMIO adresi (Genellikle Zilog tabanlı bir UART)
    pub console_addr: usize,
    /// Bellek Başlangıç Adresi
    pub ram_start: usize,
    /// Toplam Bellek Boyutu (bayt)
    pub ram_size: usize,
    /// Kesme Kontrolcüsü (UPA/JBUS/EBus Interrupt Controller) adresi
    pub interrupt_controller_addr: usize,
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
    /// SPARC V9'da, bu adres genellikle OpenPROM tarafından bir kayıtta iletilir.
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
        // SPARC V9'da OpenPROM'dan okunan bilgiler FDT formatına benzer.

        // Simülasyon: SPARC V9 platformunda tipik donanım adresleri (T-Series veya sun4u'ya benzer)
        let config = HardwareConfig {
            // Seri Port (UART) adresi (Örn: 16550/Zilog Uart, yüksek MMIO adresleri)
            console_addr: 0xFE00_0000, 
            
            // Bellek bilgisi (genellikle 'memory' düğümünden alınır)
            ram_start: 0x0000_0000, 
            ram_size: 2048 * 1024 * 1024, // 2GB
            
            // Kesme Kontrolcüsü adresi (Temsili)
            interrupt_controller_addr: 0xFD00_0000, 
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
        // serial_println! macro'sunun arch/sparcv9/console.rs'te tanımlandığını varsayıyoruz.
        serial_println!("[DTB-SPARCV9] DTB adresi bilinmiyor (0x0).");
        return;
    }
    
    // Güvenlik: Adresin geçerli bir bellek aralığında olduğunu varsayıyoruz.
    let header_slice = unsafe { 
        slice::from_raw_parts(dtb_addr as *const u32, 4) 
    };

    // Rust'ta Big Endian (dtb formatı) okuması için u32::from_be kullanılır.
    let magic = u32::from_be(header_slice[0]);

    serial_println!("[DTB-SPARCV9] DTB Adresi: {:#x}", dtb_addr);
    serial_println!("[DTB-SPARCV9] Magic (Beklenen 0xd00dfeed): {:#x}", magic);
    
    // Ayrıştırılmış yapılandırma varsa onu da bas
    if let Ok(config) = DtbParser::get_config() {
        serial_println!("[DTB-SPARCV9] -> Konsol Adresi: {:#x}", config.console_addr);
        serial_println!("[DTB-SPARCV9] -> Kesme Kontrolcü Adresi: {:#x}", config.interrupt_controller_addr);
        serial_println!("[DTB-SPARCV9] -> RAM Boyutu: {} MB", config.ram_size / 1024 / 1024);
    } else {
        serial_println!("[DTB-SPARCV9] -> Yapılandırma henüz ayrıştırılmadı.");
    }
}