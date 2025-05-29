use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{sleep, yield_now};
use std::time::Duration;
use clap::Parser;
use chrono::Local;
use nvml_wrapper::enums::device::UsedGpuMemory;
use nvml_wrapper::Nvml;
use nvml_wrapper::struct_wrappers::device::MemoryInfo;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    process_id: u32,
    #[clap(short, long, default_value_t = 0)]
    device_index: u32,
    #[clap(short = 't', long, default_value_t = 1000)]
    milli_seconds: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Args::parse();

    let n = Nvml::init()
        .expect(r#"NVML Error occurred.
        Please see https://docs.nvidia.com/pdf/NVML_API_Reference_Guide.pdf (enum nvmlReturn_t) for more info.
        Try:
        * `sudo apt install libnvidia-ml-dev`
        * reboot
        * `sudo dmesg`
"#);
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
        let MemoryInfo { total, used, free, .. } = d.memory_info().unwrap();
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
        sleep(Duration::from_millis(args.milli_seconds as u64));
    }
    println!("Received Ctrl-C");
    n.shutdown()?;
    Ok(())
}
