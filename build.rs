
fn main() {
    let mut b = freertos_cargo_build::Builder::new();

    // Path to FreeRTOS kernel or set ENV "FREERTOS_SRC" instead
    b.freertos("FreeRTOS-Kernel");
    b.freertos_config("include");       // Location of `FreeRTOSConfig.h`
    b.freertos_port("GCC/ARM_CM4F".to_string()); // Port dir relativ to 'FreeRTOS-Kernel/portable'
    // b.heap("heap4.c".to_string());              // Set the heap_?.c allocator to use from
    // 'FreeRTOS-Kernel/portable/MemMang' (Default: heap_4.c)

    // b.get_cc().file("More.c");   // Optional additional C-Code to be compiled

    b.compile().unwrap_or_else(|e| { panic!("{}", e.to_string()) });
}