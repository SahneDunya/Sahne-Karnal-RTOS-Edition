// src/arch/amd64/time.rs
// AMD64 (x86_64) mimarisine özgü zamanlama (time) işlevleri.

use core::arch::asm;
use crate::serial_println;
// Platforma özel G/Ç fonksiyonları için yer tutucu
use super::platformmod::io; 

/// TSC'den okunan ham döngü sayısını temsil eder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycles(pub u64);

// -----------------------------------------------------------------------------
// Time-Stamp Counter (TSC) İşlevleri
// -----------------------------------------------------------------------------

/// TSC (Time-Stamp Counter) yazmacını okur ve ham döngü sayısını döndürür.
///
/// TSC, işlemcinin açılışından itibaren geçen saat döngüsü sayısını tutar.
/// # Güvenlik Notu
/// Güvenilir ölçüm için, TSC'den önce ve sonra bir seri hale getirme 
/// (serialization) talimatı (örneğin `CPUID` veya `RDTSCP`) kullanmak daha iyidir.
/// Ancak basitlik için sadece `RDTSC` kullanılır.
#[inline(always)]
pub fn read_tsc() -> Cycles {
    let low: u32;
    let high: u32;
    
    // RDTSC talimatı, TSC'nin düşük 32 bitini EAX'e, yüksek 32 bitini EDX'e yazar.
    unsafe {
        asm!(
            "rdtsc",
            // EDX'i high'a, EAX'i low'a kaydet.
            out("eax") low,
            out("edx") high,
            // TSC'nin çalışmasını etkilemez
            options(nomem, nostack, preserves_flags)
        );
    }
    
    // 64-bit değeri oluştur: (high << 32) | low
    Cycles(((high as u64) << 32) | (low as u64))
}


/// TSC yazmacını okur ve sonuçları seri hale getirilmiş (serialized) olarak döndürür.
///
/// `RDTSCP`, hem TSC'yi okur hem de tüm önceki talimatların tamamlanmasını 
/// (seri hale gelmesini) sağlar, bu da daha doğru ölçüm sağlar.
/// Ayrıca `AUX` yazmacına ek bilgi yazar (genellikle CPU kimliği).
/// 
/// # Güvenlik Notu
/// `RDTSCP` kullanmadan önce işlemcinin bu özelliği desteklediğini (CPUID bayrağı)
/// kontrol etmek gerekir. Ancak bu bir `no-std` NanoKernel olduğu için basitleştirilmiştir.
#[inline(always)]
pub fn read_tscp() -> Cycles {
    let low: u32;
    let high: u32;
    let _aux: u32; // İşlemci kimliği veya benzeri bilgi
    
    unsafe {
        asm!(
            "rdtscp",
            // EDX:EAX -> high:low
            // ECX -> aux
            out("eax") low,
            out("edx") high,
            out("ecx") _aux,
            options(nomem, nostack, preserves_flags)
        );
    }
    
    Cycles(((high as u64) << 32) | (low as u64))
}


// -----------------------------------------------------------------------------
// Temel Zaman İşlevleri
// -----------------------------------------------------------------------------

/// Gecikmeli bir bekleme (busy-wait) döngüsü gerçekleştirir.
///
/// **UYARI:** Bu, çok kaba ve işlemci gücü tüketen bir bekleme yöntemidir. 
/// Sadece TSC frekansı biliniyorsa veya çok kısa gecikmeler için kullanılmalıdır.
pub fn delay_cycles(cycles: u64) {
    if cycles == 0 {
        return;
    }

    let start = read_tsc().0;
    let end = start.checked_add(cycles).unwrap_or(u64::MAX);

    while read_tsc().0 < end {
        // İşlemciyi uyarmak için HLT talimatını kullanmak yerine, 
        // daha yumuşak bir bekleme için PAUSE (rep nop) kullanılabilir.
        unsafe {
            io::pause(); // Temsili bir PAUSE (rep nop) talimatı çağrısı
        }
    }
}


/// TSC'yi kullanarak kernel zamanlama altyapısını başlatır.
///
/// Gerçek bir çekirdekte, bu fonksiyon TSC frekansını (MHz) hesaplar 
/// ve global bir değişkene kaydeder.
pub fn initialize_time_system() {
    serial_println!("[TIME] AMD64 Zamanlama Modülü Başlatılıyor...");
    
    // Gerçek bir başlatma:
    // 1. HPET veya PIT kullanarak sabit bir süre bekle.
    // 2. Bu süre zarfında TSC'yi oku.
    // 3. TSC sayımını geçen süreye bölerek TSC frekansını (döngü/sn) hesapla.
    
    let current_cycles = read_tsc();
    serial_println!("[TIME] TSC Başlangıç Değeri: {:#x}", current_cycles.0);
    
    // Bu değer, frekans hesaplanana kadar anlamsızdır.
}

// Platforma özel G/Ç fonksiyonları için bir yer tutucu (main.rs veya platformmod.rs'de tamamlanmalıdır)
mod io {
    use core::arch::asm;
    
    #[inline(always)]
    pub unsafe fn pause() {
        // PAUSE (rep nop) talimatı: Hyper-threading/SMT ortamlarında 
        // busy-wait döngülerinde işlemciyi rahatlatmak için kullanılır.
        asm!("pause", options(nomem, nostack, preserves_flags));
    }
}