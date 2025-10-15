use crate::platform::PlatformManager;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

// SPARC V9 sistemlerinde yaygın olarak kullanılan 
// 16550 uyumlu UART'ın TEMEL MMIO adresi.
// Not: Gerçek donanımınızda bu adres (özellikle PROM tarafından belirlenir) farklı olacaktır!
const UART_MMIO_ADDR: usize = 0xFE00_0000; // Temsili bir MMIO adresi (genellikle yüksek adresler)

// 16550 Uyumlu UART Yazmaçları için Ofsetler
const DATA_REGISTER_OFFSET: usize = 0x00;        // Veri (TX/RX)
const LINE_STATUS_REGISTER_OFFSET: usize = 0x05; // Hat Durumu (TX Boş mu?)
const FIFO_CTRL_OFFSET: usize = 0x02;            // FIFO Kontrol
const LINE_CTRL_OFFSET: usize = 0x03;            // Hat Kontrol

/// SPARC V9 MMIO UART'ı yöneten yapı.
///
/// Bu yapı statiktir ve çıktı deterministik olmalıdır.
pub struct Uart;

// UART'ın başlatılıp başlatılmadığını izler (Atomik Bayrak)
static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);

impl Uart {
    /// UART yazmacının tam MMIO adresini hesaplar.
    fn get_register_addr(offset: usize) -> usize {
        // Güvenli toplama işlemi
        UART_MMIO_ADDR.checked_add(offset).unwrap_or_else(|| 
            panic!("UART MMIO adresi taşması")
        )
    }

    /// UART'ı temel bir konfigürasyonla başlatır (8N1, FIFO açık).
    pub fn init() {
        if IS_INITIALIZED.load(Ordering::Acquire) {
            return; // Zaten başlatılmışsa tekrar başlatma
        }
        
        let platform_write = |offset: usize, data: u8| unsafe {
            PlatformManager::write_byte_to_address(Self::get_register_addr(offset), data)
        };
        
        // --- UART Konfigürasyonu (16550 Uyumlu) ---
        
        // 1. DLAB'ı (Divisor Latch Access Bit) ayarla (Baud Hızı için)
        platform_write(LINE_CTRL_OFFSET, 0x80);

        // 2. Baud Hızını Ayarla (Örn: 115200 baud -> Bölücü 1, Varsayılan)
        platform_write(DATA_REGISTER_OFFSET + 0, 0x01); // Bölücü Alt Bayt (LSB)
        platform_write(DATA_REGISTER_OFFSET + 1, 0x00); // Bölücü Üst Bayt (MSB)

        // 3. Hat Kontrol Yazmacını Ayarla (8 Veri Biti, 1 Stop Biti, Parite Yok)
        // DLAB'ı sıfırla (0x03 = 8N1 konfigürasyonu)
        platform_write(LINE_CTRL_OFFSET, 0x03);

        // 4. FIFO'yu Etkinleştir ve Temizle (0xC7 = FIFO etkin, alıcı/gönderici temizle)
        platform_write(FIFO_CTRL_OFFSET, 0xC7); 

        // 5. Kesme Kontrol Yazmacını sıfırla (Tüm kesmeleri kapat)
        platform_write(DATA_REGISTER_OFFSET + 1, 0x00); 

        IS_INITIALIZED.store(true, Ordering::Release);
    }
    
    /// UART'ın boş olup olmadığını kontrol eder (göndermeye hazır mı?).
    ///
    /// # Güvenlik Notu
    /// MMIO okuma işlemi olduğu için `unsafe` gerektirir.
    fn is_transmit_empty() -> bool {
        let status = unsafe { 
            // PlatformManager'ı kullanarak Hat Durumu Yazmacını oku (MMIO)
            PlatformManager::read_byte_from_address(
                Self::get_register_addr(LINE_STATUS_REGISTER_OFFSET)
            ) 
        };
        
        // 16550 Uyumlu: Bit 5 (THRE - Transmitter Holding Register Empty)
        (status & 0x20) == 0x20 // Eğer bit 5 ayarlanmışsa (1) -> Boş
    }

    /// UART'a bir bayt yazar.
    pub fn write_byte(byte: u8) {
        // Portun boş olmasını bekle (Busy-Waiting)
        while !Self::is_transmit_empty() {}

        // Veri Yazmacına baytı yaz
        unsafe {
            PlatformManager::write_byte_from_address(
                Self::get_register_addr(DATA_REGISTER_OFFSET), 
                byte
            )
        }
    }
}

/// `core::fmt::Write` trait'ini uygulayarak formatlı çıktıya izin verir.
impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            match byte {
                // `\n` (LF) gördüğünde `\r\n` (CRLF) olarak gönder
                b'\n' => {
                    self.write_byte(b'\r');
                    self.write_byte(b'\n');
                }
                // Diğer karakterleri doğrudan gönder
                _ => self.write_byte(byte),
            }
        }
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// GENEL KONSOL ÇIKTI FONKSİYONLARI (Makrolar)
// -----------------------------------------------------------------------------

/// Çekirdek içerisindeki tüm formatlı çıktı çağrılarını yakalayan makro.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        // Tam yolu kullanmak önemli
        use $crate::arch::sparcv9::console::Uart; 

        // Not: Gerçek bir çekirdekte, bu kısım yarış koşullarını önlemek için 
        // Spinlock ile korunmalı ya da Kesmeler devre dışı bırakılmalıdır.
        unsafe {
             let _ = write!($crate::arch::sparcv9::console::Uart, $($arg)*);
        }
    });
}

/// Yeni satır ekleyen çıktı makrosu.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}

// Makroların kök kütüphaneden (lib.rs) dışa aktarılması gerektiğini unutmayın.