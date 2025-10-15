// src/arch/openrisc64/task.rs
// OpenRISC 64 (OR64) mimarisine özgü görev (task) ve bağlam (context) yönetimi.

use core::arch::asm;
use crate::serial_println;

/// Görev bağlamını (task context) saklamak için kullanılan yapı.
/// OR64'te Callee-Saved (çağrılan tarafından korunan) yazmaçlar:
/// r9-r20 (s0-s11), r1 (LR), r2 (SP).
#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskContext {
    // Callee-Saved GPR'lar (r9 - r20) - 12 adet
    r9: u64,  // s0
    r10: u64, // s1
    r11: u64, // s2
    r12: u64, // s3
    r13: u64, // s4
    r14: u64, // s5
    r15: u64, // s6
    r16: u64, // s7
    r17: u64, // s8
    r18: u64, // s9
    r19: u64, // s10
    r20: u64, // s11
    
    // Link Register (r1) - Görev anahtarlamadan sonra geri döneceği adres (PC).
    r1: u64,  // LR (Link Register)
    
    // Yığın İşaretçisi (r2) - Görevin yeni yığınının adresi.
    r2: u64,  // SP (Stack Pointer) 
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
            
            // r2 (sp), yığının üstü olarak ayarlanır.
            r2: stack_top,
            
            // r1 (LR) ve pc (entry_point), görevin ilk başlayacağı adres olarak ayarlanır.
            // Anahtarlama `l.jr r1` ile görev döndüğünde, bu r1'e zıplayacaktır.
            r1: entry_point, 
        }
    }

    /// Bir bağlam anahtarlama işlemi sırasında yazmaç durumunu kaydetmek ve 
    /// yeni görevden yazmaç durumunu yüklemek için kullanılan harici assembly fonksiyonu.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon, C ABI'sine uymalıdır: `fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext)`.
    /// Argümanlar r3 ve r4'e (a0 ve a1) geçirilir.
    #[inline(always)]
    pub unsafe fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext) {
        serial_println!("[TASK] Bağlam Anahtarlama: {:#x} -> {:#x}", 
                        old_context as u64, new_context as u64);

        asm!(
            // --------------------- Mevcut Görevin Durumunu Kaydet ---------------------
            // r3: old_context, r4: new_context (C ABI'de a0 ve a1)
            
            // 1. Callee-Saved GPR'ları TaskContext'e kaydet (r9-r20)
            // l.sd rN, offset(r3) (Store Doubleword)
            
            // r9 offset 0'da başlar. 12 yazmaç = 96 bayt.
            "l.sd r9, 0(r3)",   // s0
            "l.sd r10, 8(r3)",  // s1
            "l.sd r11, 16(r3)", // s2
            "l.sd r12, 24(r3)", // s3
            "l.sd r13, 32(r3)", // s4
            "l.sd r14, 40(r3)", // s5
            "l.sd r15, 48(r3)", // s6
            "l.sd r16, 56(r3)", // s7
            "l.sd r17, 64(r3)", // s8
            "l.sd r18, 72(r3)", // s9
            "l.sd r19, 80(r3)", // s10
            "l.sd r20, 88(r3)", // s11
            
            // 2. r1 (LR) ve r2 (SP) kaydet.
            // r1 (LR) offset 96, r2 (SP) offset 104.
            "l.sd r1, 96(r3)",  // LR
            "l.sd r2, 104(r3)", // SP
            
            // --------------------- Yeni Görevin Durumunu Yükle ---------------------
            // r4: new_context
            
            // 1. Yeni r2 (SP) yükle (Offset 104)
            "l.ld r2, 104(r4)",  
            
            // 2. Callee-Saved GPR'ları yükle (r9-r20) (Offset 0)
            "l.ld r9, 0(r4)", 
            "l.ld r10, 8(r4)",
            "l.ld r11, 16(r4)",
            "l.ld r12, 24(r4)",
            "l.ld r13, 32(r4)",
            "l.ld r14, 40(r4)",
            "l.ld r15, 48(r4)",
            "l.ld r16, 56(r4)",
            "l.ld r17, 64(r4)",
            "l.ld r18, 72(r4)",
            "l.ld r19, 80(r4)",
            "l.ld r20, 88(r4)",
            
            // 3. Yeni r1 (LR) yükle (Offset 96)
            "l.ld r1, 96(r4)",
            
            // 4. Yeni görevin giriş noktasına zıpla (l.jr r1)
            "l.jr r1", // Jump Register (r1 = LR). Görevi başlatır/devam ettirir.
            
            in("r3") old_context,
            in("r4") new_context,
            // r5-r8 (a2-a5) caller-saved
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
/// * `func`: Görevin gerçek giriş noktası (başlatılacak fonksiyonun adresi, r3'te olmalı).
/// * `arg`: Göreve geçirilen u64 formatında argüman (r4'te olmalı).
extern "C" fn task_entry(func: u64, arg: u64) -> ! {
    serial_println!("[TASK] Yeni Görev Başlatılıyor. Argüman: {:#x}", arg);

    // Fonksiyon işaretçisini (u64) gerçek fonksiyona dönüştür.
    // OR64 C ABI'sinde ilk argüman r3 (a0)'ta beklenir.
    let entry_func: fn(u64) = unsafe { 
        core::mem::transmute(func as *const ()) 
    };

    // Gerçek görev fonksiyonunu çağır (arg, r4 (a1)'da beklenir)
    entry_func(arg);

    // Görev tamamlandı. Normalde bu noktaya gelinmemelidir.
    serial_println!("[TASK] Görev Tamamlandı! İşlemci durduruluyor.");
    
    // Görev tamamlandığında, çekirdek durdurulmalıdır.
    loop {
        unsafe {
            io::idle(); // OpenRISC'te bekleme talimatı
        }
    }
}


// -----------------------------------------------------------------------------
// Deneme/Başlatma Fonksiyonu
// -----------------------------------------------------------------------------

/// Deneme amacıyla statik bir görev başlatma fonksiyonu.
pub fn initialize_tasking() {
    serial_println!("[TASK] OpenRISC 64 Görev Modülü Başlatılıyor...");
    
    // Örnek: Görev giriş noktasının adresi
    let entry_point_addr = task_entry as *const (); 
    
    serial_println!("[TASK] Task Entry Adresi: {:#x}", entry_point_addr as u64);
}