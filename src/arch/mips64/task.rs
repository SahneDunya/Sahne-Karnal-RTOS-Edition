// src/arch/mips64/task.rs
// MIPS 64 (MIPS64) mimarisine özgü görev (task) ve bağlam (context) yönetimi.

use core::arch::asm;
use crate::serial_println;

/// Görev bağlamını (task context) saklamak için kullanılan yapı.
/// MIPS64'te Callee-Saved (çağrılan tarafından korunan) yazmaçlar:
/// s0-s7 (r16-r23), s8 (r30 - Frame Pointer), ra (r31 - Return Address).
#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskContext {
    // Callee-Saved GPR'lar (r16 - r23) - 8 adet (s0-s7)
    r16: u64, // s0
    r17: u64, // s1
    r18: u64, // s2
    r19: u64, // s3
    r20: u64, // s4
    r21: u64, // s5
    r22: u64, // s6
    r23: u64, // s7
    
    // Frame Pointer (r30) - s8
    r30: u64, // s8/fp
    
    // Link Register (r31) - Görev anahtarlamadan sonra geri döneceği adres (PC).
    r31: u64, // ra (Return Address)
    
    // Yığın İşaretçisi (r29) - Görevin yeni yığınının adresi.
    r29: u64, // sp (Stack Pointer) 
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
            
            // r29 (sp), yığının üstü olarak ayarlanır.
            r29: stack_top,
            
            // r31 (ra) ve pc (entry_point), görevin ilk başlayacağı adres olarak ayarlanır.
            // Anahtarlama `jr ra` ile görev döndüğünde, bu ra'ya zıplayacaktır.
            r31: entry_point, 
        }
    }

    /// Bir bağlam anahtarlama işlemi sırasında yazmaç durumunu kaydetmek ve 
    /// yeni görevden yazmaç durumunu yüklemek için kullanılan harici assembly fonksiyonu.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon, C ABI'sine uymalıdır: `fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext)`.
    /// Argümanlar r4 ve r5'e (a0 ve a1) geçirilir.
    #[inline(always)]
    pub unsafe fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext) {
        serial_println!("[TASK] Bağlam Anahtarlama: {:#x} -> {:#x}", 
                        old_context as u64, new_context as u64);

        asm!(
            // --------------------- Mevcut Görevin Durumunu Kaydet ---------------------
            // r4: old_context, r5: new_context (C ABI'de a0 ve a1)
            
            // 1. Callee-Saved GPR'ları TaskContext'e kaydet (r16-r23, r30, r31)
            // sd rN, offset(r4) (Store Doubleword)
            
            // r16 offset 0'da başlar. 10 yazmaç = 80 bayt.
            "sd r16, 0(r4)",   // s0
            "sd r17, 8(r4)",   // s1
            "sd r18, 16(r4)",  // s2
            "sd r19, 24(r4)",  // s3
            "sd r20, 32(r4)",  // s4
            "sd r21, 40(r4)",  // s5
            "sd r22, 48(r4)",  // s6
            "sd r23, 56(r4)",  // s7
            "sd r30, 64(r4)",  // s8/fp
            "sd r31, 72(r4)",  // ra
            
            // 2. r29 (sp) kaydet. Offset 80.
            "sd r29, 80(r4)",  // sp
            
            // --------------------- Yeni Görevin Durumunu Yükle ---------------------
            // r5: new_context
            
            // 1. Yeni r29 (sp) yükle (Offset 80)
            "ld r29, 80(r5)",  
            
            // 2. Callee-Saved GPR'ları yükle (r16-r23, r30, r31) (Offset 0)
            "ld r16, 0(r5)", 
            "ld r17, 8(r5)",
            "ld r18, 16(r5)",
            "ld r19, 24(r5)",
            "ld r20, 32(r5)",
            "ld r21, 40(r5)",
            "ld r22, 48(r5)",
            "ld r23, 56(r5)",
            "ld r30, 64(r5)",
            "ld r31, 72(r5)",
            
            // 3. Yeni görevin giriş noktasına zıpla (jr ra)
            "jr r31", // Jump Register (r31 = ra). Görevi başlatır/devam ettirir.
            
            in("r4") old_context,
            in("r5") new_context,
            // r6-r11 (a2-a7) caller-saved
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
/// * `func`: Görevin gerçek giriş noktası (başlatılacak fonksiyonun adresi, r4'te olmalı).
/// * `arg`: Göreve geçirilen u64 formatında argüman (r5'te olmalı).
extern "C" fn task_entry(func: u64, arg: u64) -> ! {
    serial_println!("[TASK] Yeni Görev Başlatılıyor. Argüman: {:#x}", arg);

    // Fonksiyon işaretçisini (u64) gerçek fonksiyona dönüştür.
    // MIPS64 C ABI'sinde ilk argüman r4 (a0)'ta beklenir.
    let entry_func: fn(u64) = unsafe { 
        core::mem::transmute(func as *const ()) 
    };

    // Gerçek görev fonksiyonunu çağır (arg, r5 (a1)'da beklenir)
    entry_func(arg);

    // Görev tamamlandı. Normalde bu noktaya gelinmemelidir.
    serial_println!("[TASK] Görev Tamamlandı! İşlemci durduruluyor.");
    
    // Görev tamamlandığında, çekirdek durdurulmalıdır.
    loop {
        unsafe {
            io::wait(); // MIPS'te bekleme talimatı
        }
    }
}


// -----------------------------------------------------------------------------
// Deneme/Başlatma Fonksiyonu
// -----------------------------------------------------------------------------

/// Deneme amacıyla statik bir görev başlatma fonksiyonu.
pub fn initialize_tasking() {
    serial_println!("[TASK] MIPS 64 Görev Modülü Başlatılıyor...");
    
    // Örnek: Görev giriş noktasının adresi
    let entry_point_addr = task_entry as *const (); 
    
    serial_println!("[TASK] Task Entry Adresi: {:#x}", entry_point_addr as u64);
}