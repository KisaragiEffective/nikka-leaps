use std::any::Any;
use std::error::Error;
use std::mem;
use std::os::raw;
use std::os::raw::c_uint;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;
use chrono::Local;
use libloading::{Library, Symbol};
use nvml_wrapper::enums::device::UsedGpuMemory;
use nvml_wrapper::error::{nvml_sym, nvml_try, NvmlError};
use nvml_wrapper::{Device, Nvml};
use nvml_wrapper::struct_wrappers::device::{MemoryInfo, ProcessInfo};
use nvml_wrapper_sys::bindings::{nvmlDevice_t, nvmlProcessInfo_t, nvmlReturn_t};
use once_cell::sync::Lazy;

trait GetRunningGraphicsProcessesV2 {
    fn running_graphics_processes_v2(&self) -> Result<Vec<ProcessInfo>, NvmlError>;
}

type GetProcessV2Sig = unsafe extern "C" fn(
    device: nvmlDevice_t,
    #[allow(non_snake_case)]
    infoCount: *mut c_uint,
    infos: *mut nvmlProcessInfo_t,
) -> nvmlReturn_t;

static GET_PS_V2: Lazy<GetProcessV2Sig> = Lazy::new(|| {
    unsafe {
        let lib = Library::new("libnvidia-ml.so").unwrap();
        let f: GetProcessV2Sig = lib.get(b"nvmlDeviceGetGraphicsRunningProcesses_v2\0").map(|a| *a).unwrap();
        f
    }
});

impl GetRunningGraphicsProcessesV2 for Device<'_> {
    fn running_graphics_processes_v2(&self) -> Result<Vec<ProcessInfo>, NvmlError> {
        let nvmlDeviceGetGraphicsRunningProcesses_v2 = *GET_PS_V2;

        let sym = nvml_sym(Ok(&nvmlDeviceGetGraphicsRunningProcesses_v2))?;

        unsafe {
            let mut count: c_uint = match self.running_graphics_processes_count()? {
                0 => return Ok(vec![]),
                value => value,
            };
            // Add a bit of headroom in case more processes are launched in
            // between the above call to get the expected count and the time we
            // actually make the call to get data below.
            count += 5;
            let mut processes: Vec<nvmlProcessInfo_t> = vec![mem::zeroed(); count as usize];

            struct X<'a> {
                device: nvmlDevice_t,
                nvml: &'a ()
            }

            nvml_try(sym(mem::transmute::<&_, &X<'_>>(self).device, &mut count, processes.as_mut_ptr()))?;
            processes.truncate(count as usize);

            Ok(processes.into_iter().map(ProcessInfo::from).collect())
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let n = Nvml::init().unwrap();
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
        let process_info = d.running_graphics_processes_v2()?;
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
