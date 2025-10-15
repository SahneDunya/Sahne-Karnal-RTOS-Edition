// src/arch/rv64i/task.rs
// RISC-V 64 (RV64I) mimarisine özgü görev (task) ve bağlam (context) yönetimi.

use core::arch::asm;
use crate::serial_println;

/// Görev bağlamını (task context) saklamak için kullanılan yapı.
/// RISC-V 64 ABI'de Callee-Saved yazmaçlar:
/// s0 (x8), s1 (x9), s2-s11 (x18-x27) - Toplam 12 adet.
/// Ayrıca ra (x1) ve sp (x2) da bağlamda tutulmalıdır.
#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskContext {
    // Callee-Saved GPR'lar (s0-s11: x8, x9, x18-x27) - 12 adet
    x8_s0: u64,
    x9_s1: u64,
    x18_s2: u64,
    x19_s3: u64,
    x20_s4: u64,
    x21_s5: u64,
    x22_s6: u64,
    x23_s7: u64,
    x24_s8: u64,
    x25_s9: u64,
    x26_s10: u64,
    x27_s11: u64,
    
    // Bağlam Anahtarlama için kritik yazmaçlar
    x1_ra: u64,  // ra (Return Address) - PC görevi görür
    x2_sp: u64,  // sp (Stack Pointer)
}

impl TaskContext {
    /// Yeni bir görev bağlamı oluşturur.
    /// 
    /// # Argümanlar
    /// * `stack_top`: Görevin yığınının en üst adresi.
    /// * `entry_point`: Görevin başlayacağı fonksiyonun adresi.
    pub fn new(stack_top: u64, entry_point: u64) -> Self {
        // Yeni bir görev başlatıldığında, anahtarlama kodu bu yapıyı yükler.
        Self {
            // Callee-Saved yazmaçlar sıfırlanır.
            ..Default::default()
            
            // x2_sp, yığının üstü olarak ayarlanır.
            x2_sp: stack_top,
            
            // x1_ra, görevin ilk başlayacağı adres olarak ayarlanır.
            // Anahtarlama `jr ra` ile görev döndüğünde, bu ra'ya zıplayacaktır.
            x1_ra: entry_point, 
        }
    }

    /// Bir bağlam anahtarlama işlemi sırasında yazmaç durumunu kaydetmek ve 
    /// yeni görevden yazmaç durumunu yüklemek için kullanılan harici assembly fonksiyonu.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon, C ABI'sine uymalıdır: `fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext)`.
    /// Argümanlar x10 ve x11'e (a0 ve a1) geçirilir.
    #[inline(always)]
    pub unsafe fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext) {
        serial_println!("[TASK] Bağlam Anahtarlama: {:#x} -> {:#x}", 
                        old_context as u64, new_context as u64);

        asm!(
            // --------------------- Mevcut Görevin Durumunu Kaydet ---------------------
            // x10: old_context, x11: new_context (C ABI'de a0 ve a1)
            
            // 1. Callee-Saved GPR'ları TaskContext'e kaydet (s0-s11)
            // sd xN, offset(x10) (Store Doubleword)
            
            // x8_s0 offset 0'da başlar. 12 yazmaç = 96 bayt.
            "sd x8, 0(x10)",   // s0 (x8)
            "sd x9, 8(x10)",   // s1 (x9)
            "sd x18, 16(x10)", // s2 (x18)
            "sd x19, 24(x10)", // s3 (x19)
            "sd x20, 32(x10)", // s4 (x20)
            "sd x21, 40(x10)", // s5 (x21)
            "sd x22, 48(x10)", // s6 (x22)
            "sd x23, 56(x10)", // s7 (x23)
            "sd x24, 64(x10)", // s8 (x24)
            "sd x25, 72(x10)", // s9 (x25)
            "sd x26, 80(x10)", // s10 (x26)
            "sd x27, 88(x10)", // s11 (x27)
            
            // 2. ra (x1) ve sp (x2) kaydet.
            // ra (x1) offset 96, sp (x2) offset 104.
            "sd x1, 96(x10)",  // ra
            "sd x2, 104(x10)", // sp
            
            // --------------------- Yeni Görevin Durumunu Yükle ---------------------
            // x11: new_context
            
            // 1. Yeni x2 (sp) yükle (Offset 104)
            "ld x2, 104(x11)",  
            
            // 2. Callee-Saved GPR'ları yükle (s0-s11) (Offset 0)
            "ld x8, 0(x11)", 
            "ld x9, 8(x11)",
            "ld x18, 16(x11)",
            "ld x19, 24(x11)",
            "ld x20, 32(x11)",
            "ld x21, 40(x11)",
            "ld x22, 48(x11)",
            "ld x23, 56(x11)",
            "ld x24, 64(x11)",
            "ld x25, 72(x11)",
            "ld x26, 80(x11)",
            "ld x27, 88(x11)",
            
            // 3. Yeni x1 (ra) yükle (Offset 96)
            "ld x1, 96(x11)",
            
            // 4. Yeni görevin giriş noktasına zıpla (jr ra)
            "jr x1", // Jump Register (x1 = ra). Görevi başlatır/devam ettirir.
            
            in("x10") old_context,
            in("x11") new_context,
            // x12-x17 (a2-a7) caller-saved
            options(noreturn, preserves_flags)
        );
    }
}


// -----------------------------------------------------------------------------
// Görev Başlatma (Task Entry)
// -----------------------------------------------------------------------------

/// Yeni görevlerin ilk başladığı yer. 
/// Görev, bu fonksiyonun sonundan asla dönmemelidir (return).
///
/// # Argümanlar
/// * `func`: Görevin gerçek giriş noktası (başlatılacak fonksiyonun adresi, x10'da olmalı).
/// * `arg`: Göreve geçirilen u64 formatında argüman (x11'de olmalı).
extern "C" fn task_entry(func: u64, arg: u64) -> ! {
    serial_println!("[TASK] Yeni Görev Başlatılıyor. Argüman: {:#x}", arg);

    // Fonksiyon işaretçisini (u64) gerçek fonksiyona dönüştür.
    // RV64 C ABI'sinde ilk argüman x10 (a0)'da beklenir.
    let entry_func: fn(u64) = unsafe { 
        core::mem::transmute(func as *const ()) 
    };

    // Gerçek görev fonksiyonunu çağır (arg, x11 (a1)'da beklenir)
    entry_func(arg);

    // Görev tamamlandı. Normalde bu noktaya gelinmemelidir.
    serial_println!("[TASK] Görev Tamamlandı! İşlemci durduruluyor.");
    
    // Görev tamamlandığında, çekirdek durdurulmalıdır.
    loop {
        unsafe {
            io::wfi(); // RISC-V'de bekleme talimatı
        }
    }
}


// -----------------------------------------------------------------------------
// Deneme/Başlatma Fonksiyonu
// -----------------------------------------------------------------------------

/// Deneme amacıyla statik bir görev başlatma fonksiyonu.
pub fn initialize_tasking() {
    serial_println!("[TASK] RISC-V 64 Görev Modülü Başlatılıyor...");
    
    // Örnek: Görev giriş noktasının adresi
    let entry_point_addr = task_entry as *const (); 
    
    serial_println!("[TASK] Task Entry Adresi: {:#x}", entry_point_addr as u64);
}