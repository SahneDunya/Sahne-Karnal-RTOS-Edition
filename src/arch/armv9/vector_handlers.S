.global vector_table_sync_spx
vector_table_sync_spx:
    // 1. Tüm GPR'ları yığına kaydet
    // ...
    // 2. SPSR_EL1 ve ELR_EL1 kaydet
    mrs x20, SPSR_EL1
    mrs x21, ELR_EL1
    stp x20, x21, [sp, #-16]! // Kaydet

    // 3. ESR_EL1 oku ve parametre olarak hazırla
    mrs x0, ESR_EL1
    mov x1, sp // ExceptionContext (GPR'lar dahil yığın adresi)

    bl generic_sync_handler // Rust fonksiyonunu çağır

    // 4. Dönüş işlemleri (Geri Yükleme)
    ldp x20, x21, [sp], #16 // SPSR_EL1 ve ELR_EL1 geri yükle
    msr SPSR_EL1, x20
    msr ELR_EL1, x21
    // ... Tüm GPR'ları yığından geri yükle ...
    eret // İstisnadan dön