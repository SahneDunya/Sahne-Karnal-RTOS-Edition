use crate::platform::PlatformManager;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

// Seri Port (COM1) I/O Port Adresi. x86/AMD64'te standarttır.
const COM1_PORT: u16 = 0x3F8;

// Seri Port Yazmaçları için Ofsetler
const DATA_PORT: u16 = 0; // Veri yazma/okuma
const FIFO_CTRL_PORT: u16 = 2; // FIFO Kontrol Yazmacı
const LINE_CTRL_PORT: u16 = 3; // Hat Kontrol Yazmacı
const LINE_STATUS_PORT: u16 = 5; // Hat Durumu Yazmacı

/// AMD64 Seri Port I/O'yu yöneten yapı.
///
/// Bu yapı statiktir ve çıktı deterministik olmalıdır.
pub struct SerialPort;

// Seri Port'un zaten başlatılıp başlatılmadığını izler (Atomik Bayrak)
static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);

impl SerialPort {
    /// Seri Portu 115200 baud hızında başlatır.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon I/O port G/Ç kullandığı için `unsafe` gerektirir.
    /// Çağrılar `PlatformManager` aracılığıyla güvenli bir şekilde soyutlanmıştır.
    pub fn init() {
        if IS_INITIALIZED.load(Ordering::Acquire) {
            return; // Zaten başlatılmışsa tekrar başlatma
        }
        
        // Bu işlemler, src/arch/amd64/platformmod.rs dosyasındaki
        // write_byte_to_address fonksiyonunu kullanarak yapılmalıdır.
        let platform_write = |port: u16, data: u8| unsafe {
            // usize'a çevirme platformmod.rs içinde yapılacaktır,
            // ancak I/O portu olduğu için u16'yı hedefliyoruz.
            PlatformManager::write_byte_to_address(COM1_PORT as usize + port as usize, data)
        };
        
        // 1. Kesmeleri Kapat (Değişken Bölücüye Erişim için DLAB'ı ayarla)
        platform_write(LINE_CTRL_PORT, 0x80);

        // 2. Baud Hızını Ayarla (115200 baud) -> Bölücü 1
        platform_write(DATA_PORT + 0, 0x01); // Bölücü Alt Bayt (LSB)
        platform_write(DATA_PORT + 1, 0x00); // Bölücü Üst Bayt (MSB)

        // 3. Hat Kontrol Yazmacını Ayarla (8 Veri Biti, 1 Stop Biti, Parite Yok)
        // DLAB'ı sıfırla (0x03 = 8N1 konfigürasyonu)
        platform_write(LINE_CTRL_PORT, 0x03);

        // 4. FIFO'yu Etkinleştir (Clear FIFO'lar, FIFO etkinleştir)
        platform_write(FIFO_CTRL_PORT, 0xC7);

        // 5. Kesme Kontrol Yazmacını sıfırla (Tüm kesmeleri kapat)
        platform_write(DATA_PORT + 1, 0x00); 

        IS_INITIALIZED.store(true, Ordering::Release);
    }
    
    /// Seri Portun boş olup olmadığını kontrol eder.
    ///
    /// # Güvenlik Notu
    /// I/O Port okuma işlemi olduğu için `unsafe` gerektirir.
    fn is_transmit_empty() -> bool {
        let platform_read = |port: u16| unsafe {
            PlatformManager::read_byte_from_address(COM1_PORT as usize + port as usize)
        };
        
        // Hat Durumu Yazmacı'nın 5. bitini kontrol et (Veri İletim Kaydı Boş)
        (platform_read(LINE_STATUS_PORT) & 0x20) == 0x20
    }

    /// Seri Port'a bir bayt yazar.
    pub fn write_byte(byte: u8) {
        // Portun boş olmasını bekle (Busy-Waiting)
        while !Self::is_transmit_empty() {}

        // Veri Portuna baytı yaz
        unsafe {
            PlatformManager::write_byte_from_address(COM1_PORT as usize + DATA_PORT as usize, byte)
        }
    }
}

/// `core::fmt::Write` trait'ini uygulayarak formatlı çıktıya izin verir.
/// NanoKernel'de loglama için kritik öneme sahiptir.
impl fmt::Write for SerialPort {
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
// GENEL KONSOL ÇIKTI FONKSİYONLARI
// -----------------------------------------------------------------------------

// Güvenli G/Ç için statik bir Seri Port örneği
static mut SERIAL_PORT: SerialPort = SerialPort;

/// Çekirdek içerisindeki tüm formatlı çıktı çağrılarını yakalayan makro.
///
/// Bu makro, tüm formatlı çıktıların Seri Port'a yönlendirilmesini sağlar.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        use $crate::arch::amd64::console::SerialPort; // Tam yolu kullan

        // G/Ç işlemleri sırasında yarış koşullarını önlemek için kilit (spinlock)
        // kullanılmalıdır. Şimdilik sadece `unsafe` kullanarak doğrudan erişimi sağlıyoruz.
        // Gerçek bir çekirdekte burada bir Mutex/Spinlock çağrısı olur.
        unsafe {
             let _ = write!(SerialPort, $($arg)*);
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

// Kök kütüphane (`lib.rs`) dosyanızın bu makroları dışa aktarması gerekir.
// Örneğin: `pub use crate::arch::amd64::console::{serial_print, serial_println};`