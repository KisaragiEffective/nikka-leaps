use std::mem;
use std::os::raw::c_uint;
use std::ptr::null_mut;
use libloading::Library;
use nvml_wrapper::Device;
use nvml_wrapper::error::{nvml_try, NvmlError};
use nvml_wrapper::struct_wrappers::device::ProcessInfo;
use nvml_wrapper_sys::bindings::{nvmlDevice_t, nvmlProcessInfo_t, nvmlReturn_t};
use once_cell::sync::Lazy;

pub trait GetRunningGraphicsProcessesV2 {
    fn running_graphics_processes_v2(&self) -> Result<Vec<ProcessInfo>, NvmlError>;

    fn running_graphics_processes_count_v2(&self) -> Result<u32, NvmlError>;
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

// NOTE: implementation code in this `impl` block
// are taken from https://github.com/Cldfire/nvml-wrapper/blob/v0.8.0/nvml-wrapper/src/device.rs#L1321-L1378.
// There fore, `MIT OR Apache-2.0` (by Cldfire) applies on this part.
// See https://github.com/Cldfire/nvml-wrapper/blob/v0.8.0/LICENSE-APACHE and
// https://github.com/Cldfire/nvml-wrapper/blob/v0.8.0/LICENSE-MIT for full-text.
impl GetRunningGraphicsProcessesV2 for Device<'_> {
    fn running_graphics_processes_v2(&self) -> Result<Vec<ProcessInfo>, NvmlError> {
        let sym = *GET_PS_V2;

        let mut count: c_uint = match self.running_graphics_processes_count_v2()? {
            0 => return Ok(vec![]),
            value => value,
        };
        // Add a bit of headroom in case more processes are launched in
        // between the above call to get the expected count and the time we
        // actually make the call to get data below.
        count += 5;
        let mem = unsafe { mem::zeroed() };
        let mut processes: Vec<nvmlProcessInfo_t> = vec![mem; count as usize];

        let device = unsafe { self.handle() };
        let call = unsafe { sym(device, &mut count, processes.as_mut_ptr()) };
        nvml_try(call)?;
        processes.truncate(count as usize);

        Ok(processes.into_iter().map(ProcessInfo::from).collect())
    }

    fn running_graphics_processes_count_v2(&self) -> Result<u32, NvmlError> {
        let sym = *GET_PS_V2;

        // Indicates that we want the count
        let mut count: c_uint = 0;

        // Passing null doesn't indicate that we want the count. It's just allowed.
        let device = unsafe { self.handle() };
        let call = unsafe { sym(device, &mut count, null_mut()) };
        match call {
            nvmlReturn_enum_NVML_ERROR_INSUFFICIENT_SIZE => Ok(count),
            // If success, return 0; otherwise, return error
            other => nvml_try(other).map(|_| 0),
        }
    }
}
