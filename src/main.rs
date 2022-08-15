use std::any::Any;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;
use chrono::Local;
use nvml_wrapper::enums::device::UsedGpuMemory;
use nvml_wrapper::Nvml;
use nvml_wrapper::struct_wrappers::device::{MemoryInfo, ProcessInfo};

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let n = Nvml::init()?;
    let d = n.device_by_index(0)?;
    let m = d.memory_info()?;
    let pid = 14518;
    println!("total: {}", m.total);
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    println!("running");
    while running.load(Ordering::SeqCst) {
        let MemoryInfo { total, used, free } = d.memory_info()?;
        println!("{total} - {used} = {free}");
        let process_info = d.running_graphics_processes()?;
        let mem = {
            match process_info.into_iter().find(|a| a.pid == pid) {
                None => -1,
                Some(pi) => {
                    match pi.used_gpu_memory {
                        UsedGpuMemory::Unavailable => -1,
                        UsedGpuMemory::Used(a) => a as i32,
                    }
                }
            }
        };
        println!("{t} {mem}", t = Local::now().format("%H:%M:%S%.3f"));
        sleep(Duration::from_secs(1));
    }
    println!("Received Ctrl-C");
    n.shutdown()?;
    Ok(())
}
