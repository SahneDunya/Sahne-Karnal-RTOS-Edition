// src/arch/loongarch64/task.rs
// LoongArch 64 (LA64) mimarisine özgü görev (task) ve bağlam (context) yönetimi.

use core::arch::asm;
use crate::serial_println;

/// Görev bağlamını (task context) saklamak için kullanılan yapı.
/// LA64'te Callee-Saved (çağrılan tarafından korunan) yazmaçlar:
/// r22-r31 (S8-S15, FP, SP), r4 (ra/LR).
/// r3 (sp) ve r4 (ra/LR) bağlam anahtarlamada kritik öneme sahiptir.
#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskContext {
    // Callee-Saved GPR'lar (r22 - r31) - 10 adet
    // LoongArch ABI: r22-r30 (s8-s14, fp)
    r22: u64, // s8
    r23: u64, // s9
    r24: u64, // s10
    r25: u64, // s11
    r26: u64, // s12
    r27: u64, // s13
    r28: u64, // s14
    r29: u64, // s15 (Genellikle ekstra geçici)
    r30: u64, // fp (Frame Pointer)
    r31: u64, // tp (Thread Pointer - Genellikle Callee-Saved değil, ancak kaydedilir)
    
    // Link Register (r4) - Görev anahtarlamadan sonra geri döneceği adres (PC).
    ra: u64, // r4 (Return Address / Link Register)
    
    // Yığın İşaretçisi (r3) - Görevin yeni yığınının adresi.
    sp: u64, // r3 (Stack Pointer) 
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
            
            // sp, yığının üstü olarak ayarlanır. (LA64'te r3)
            sp: stack_top,
            
            // ra (r4) ve pc (entry_point), görevin ilk başlayacağı adres olarak ayarlanır.
            // Anahtarlama `jr ra` ile görev döndüğünde, bu ra'ya zıplayacaktır.
            ra: entry_point, 
        }
    }

    /// Bir bağlam anahtarlama işlemi sırasında yazmaç durumunu kaydetmek ve 
    /// yeni görevden yazmaç durumunu yüklemek için kullanılan harici assembly fonksiyonu.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon, C ABI'sine uymalıdır: `fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext)`.
    /// Argümanlar r5 ve r6'ya (a0 ve a1) geçirilir.
    #[inline(always)]
    pub unsafe fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext) {
        serial_println!("[TASK] Bağlam Anahtarlama: {:#x} -> {:#x}", 
                        old_context as u64, new_context as u64);

        asm!(
            // --------------------- Mevcut Görevin Durumunu Kaydet ---------------------
            // r5: old_context, r6: new_context (C ABI'de a0 ve a1)
            
            // 1. Callee-Saved GPR'ları TaskContext'e kaydet (r22-r31)
            // st.d rN, r5, #offset (Store Doubleword)
            
            // r22 offset 0'da başlar. 10 yazmaç = 80 bayt.
            "st.d r22, r5, 0", 
            "st.d r23, r5, 8",
            "st.d r24, r5, 16",
            "st.d r25, r5, 24",
            "st.d r26, r5, 32",
            "st.d r27, r5, 40",
            "st.d r28, r5, 48",
            "st.d r29, r5, 56",
            "st.d r30, r5, 64",
            "st.d r31, r5, 72",
            
            // 2. r4 (ra) ve r3 (sp) kaydet.
            // ra (r4) offset 80, sp (r3) offset 88.
            "st.d r4, r5, 80",  // ra
            "st.d r3, r5, 88",  // sp
            
            // --------------------- Yeni Görevin Durumunu Yükle ---------------------
            // r6: new_context
            
            // 1. Yeni r3 (sp) yükle (Offset 88)
            "ld.d r3, r6, 88",  
            
            // 2. Callee-Saved GPR'ları yükle (r22-r31) (Offset 0)
            "ld.d r22, r6, 0", 
            "ld.d r23, r6, 8",
            "ld.d r24, r6, 16",
            "ld.d r25, r6, 24",
            "ld.d r26, r6, 32",
            "ld.d r27, r6, 40",
            "ld.d r28, r6, 48",
            "ld.d r29, r6, 56",
            "ld.d r30, r6, 64",
            "ld.d r31, r6, 72",
            
            // 3. Yeni r4 (ra) yükle (Offset 80)
            "ld.d r4, r6, 80",
            
            // 4. Yeni görevin giriş noktasına zıpla (jr ra)
            "jr r4", // Jump Register (Görevi başlatır/devam ettirir)
            
            in("r5") old_context,
            in("r6") new_context,
            // r7-r12 (a2-a7) caller-saved
            // r1, r2, r13-r21 (t0-t9) temporaries/caller-saved
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
/// * `func`: Görevin gerçek giriş noktası (başlatılacak fonksiyonun adresi, r5'te olmalı).
/// * `arg`: Göreve geçirilen u64 formatında argüman (r6'da olmalı).
extern "C" fn task_entry(func: u64, arg: u64) -> ! {
    serial_println!("[TASK] Yeni Görev Başlatılıyor. Argüman: {:#x}", arg);

    // Fonksiyon işaretçisini (u64) gerçek fonksiyona dönüştür.
    // LA64 C ABI'sinde ilk argüman r5 (a0)'ta beklenir.
    let entry_func: fn(u64) = unsafe { 
        core::mem::transmute(func as *const ()) 
    };

    // Gerçek görev fonksiyonunu çağır (arg, r6 (a1)'da beklenir)
    entry_func(arg);

    // Görev tamamlandı. Normalde bu noktaya gelinmemelidir.
    serial_println!("[TASK] Görev Tamamlandı! İşlemci durduruluyor.");
    
    // Görev tamamlandığında, çekirdek durdurulmalıdır.
    loop {
        unsafe {
            io::idle(); // LoongArch'ta bekleme talimatı
        }
    }
}


// -----------------------------------------------------------------------------
// Deneme/Başlatma Fonksiyonu
// -----------------------------------------------------------------------------

/// Deneme amacıyla statik bir görev başlatma fonksiyonu.
pub fn initialize_tasking() {
    serial_println!("[TASK] LoongArch 64 Görev Modülü Başlatılıyor...");
    
    // Örnek: Görev giriş noktasının adresi
    let entry_point_addr = task_entry as *const (); 
    
    serial_println!("[TASK] Task Entry Adresi: {:#x}", entry_point_addr as u64);
}