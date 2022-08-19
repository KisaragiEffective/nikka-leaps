mod driver;

use std::any::Any;
use std::error::Error;
use std::os::raw;
use std::os::raw::c_uint;
use std::ptr::null_mut;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{sleep, yield_now};
use std::time::Duration;
use clap::Parser;
use chrono::Local;
use nvml_wrapper::enums::device::UsedGpuMemory;
use nvml_wrapper::Nvml;
use nvml_wrapper::struct_wrappers::device::MemoryInfo;
use crate::driver::GetRunningGraphicsProcessesV2;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    process_id: u32,
    #[clap(short, long, default_value_t = 0)]
    device_index: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Args::parse();

    let n = Nvml::init().unwrap();
    let d = n.device_by_index(args.device_index).unwrap();
    let m = d.memory_info().unwrap();
    let pid = args.process_id;
    println!("total: {}", m.total);
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    println!("running");
    while running.load(Ordering::SeqCst) {
        let MemoryInfo { total, used, free } = d.memory_info().unwrap();
        println!("{total} - {used} = {free}");
        let process_info = d.running_graphics_processes_v2().unwrap();
        let mem = {
            match process_info.into_iter().find(|a| a.pid == pid) {
                None => -1,
                Some(pi) => {
                    match pi.used_gpu_memory {
                        UsedGpuMemory::Unavailable => -1,
                        UsedGpuMemory::Used(a) => a as i64,
                    }
                }
            }
        };
        println!("{t} {mem}", t = Local::now().format("%H:%M:%S%.3f"));
        yield_now();
        sleep(Duration::from_secs(1));
    }
    println!("Received Ctrl-C");
    n.shutdown()?;
    Ok(())
}
