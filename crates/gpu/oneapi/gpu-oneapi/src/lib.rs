//! Intel oneAPI GPU backend for Shrimply
//!
//! This crate provides Intel GPU support using oneAPI Level Zero or SYCL.
//! It mirrors the CUDA backend API for consistency.

#![cfg(any(target_os = "linux", windows))]

use std::{
    ffi::c_void,
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub mod sys;

pub const DEVICE_UUID_BYTES: usize = 16;

unsafe extern "C" {
    fn shrimply_oneapi_init(flags: u32) -> sys::PIResult;
    fn shrimply_oneapi_driver_count(count: *mut u32) -> sys::PIResult;
    fn shrimply_oneapi_driver_get(driver: *mut *mut c_void, index: u32) -> sys::PIResult;
    fn shrimply_oneapi_device_count(driver: *mut c_void, count: *mut u32) -> sys::PIResult;
    fn shrimply_oneapi_device_get(
        driver: *mut c_void,
        index: u32,
        device: *mut *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_device_uuid(uuid: *mut u8, device: *mut c_void) -> sys::PIResult;
    fn shrimply_oneapi_context_create(
        context: *mut *mut c_void,
        driver: *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_queue_create(
        queue: *mut *mut c_void,
        context: *mut c_void,
        device: *mut c_void,
        flags: u32,
    ) -> sys::PIResult;
    fn shrimply_oneapi_command_list_create(
        cmdlist: *mut *mut c_void,
        context: *mut c_void,
        device: *mut c_void,
        queue: *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_mem_alloc(
        pointer: *mut *mut c_void,
        context: *mut c_void,
        bytes: usize,
    ) -> sys::PIResult;
    fn shrimply_oneapi_mem_free(pointer: *mut c_void) -> sys::PIResult;
    fn shrimply_oneapi_memcpy_htod_async(
        destination: *mut c_void,
        source: *const c_void,
        bytes: usize,
        cmdlist: *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_memcpy_dtoh_async(
        destination: *mut c_void,
        source: *mut c_void,
        bytes: usize,
        cmdlist: *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_memcpy_dtod_async(
        destination: *mut c_void,
        source: *mut c_void,
        bytes: usize,
        cmdlist: *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_memset_async(
        destination: *mut c_void,
        value: u8,
        bytes: usize,
        cmdlist: *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_queue_synchronize(queue: *mut c_void) -> sys::PIResult;
    fn shrimply_oneapi_command_list_close(cmdlist: *mut c_void) -> sys::PIResult;
    fn shrimply_oneapi_queue_execute(
        queue: *mut c_void,
        count: u32,
        cmdlists: *mut *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_event_create(
        event: *mut *mut c_void,
        context: *mut c_void,
        flags: u32,
    ) -> sys::PIResult;
    fn shrimply_oneapi_event_destroy(event: *mut c_void) -> sys::PIResult;
    fn shrimply_oneapi_event_record(
        event: *mut c_void,
        cmdlist: *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_event_synchronize(event: *mut c_void) -> sys::PIResult;
    fn shrimply_oneapi_event_elapsed(
        milliseconds: *mut f32,
        start: *mut c_void,
        end: *mut c_void,
    ) -> sys::PIResult;
    fn shrimply_oneapi_module_create(
        module: *mut *mut c_void,
        context: *mut c_void,
        device: *mut c_void,
        image: *const c_void,
        length: usize,
    ) -> sys::PIResult;
    fn shrimply_oneapi_module_destroy(module: *mut c_void) -> sys::PIResult;
    fn shrimply_oneapi_module_get_kernel(
        kernel: *mut *mut c_void,
        module: *mut c_void,
        name: *const std::os::raw::c_char,
    ) -> sys::PIResult;
    fn shrimply_oneapi_kernel_launch(
        kernel: *mut c_void,
        gx: u32,
        gy: u32,
        gz: u32,
        bx: u32,
        by: u32,
        bz: u32,
        shared: u32,
        cmdlist: *mut c_void,
        arguments: *mut *mut c_void,
    ) -> sys::PIResult;
}

fn check(result: sys::PIResult) -> Result<(), DriverError> {
    if result == sys::PI_SUCCESS {
        Ok(())
    } else {
        Err(DriverError(result))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DriverError(pub sys::PIResult);

impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "oneAPI error: {}", self.0)
    }
}

impl std::error::Error for DriverError {}

/// oneAPI context, similar to CudaContext
#[derive(Debug)]
pub struct OneApiContext {
    pub driver: *mut c_void,
    pub device: *mut c_void,
    pub context: *mut c_void,
    pub ordinal: usize,
    pub queue_count: AtomicUsize,
}

unsafe impl Send for OneApiContext {}
unsafe impl Sync for OneApiContext {}

impl OneApiContext {
    pub fn new(ordinal: usize) -> Result<Arc<Self>, DriverError> {
        check(unsafe { shrimply_oneapi_init(0) })?;
        
        let mut driver_count = 0;
        check(unsafe { shrimply_oneapi_driver_count(&mut driver_count) })?;
        
        if driver_count == 0 {
            return Err(DriverError(-1));
        }
        
        let mut driver = std::ptr::null_mut();
        check(unsafe { shrimply_oneapi_driver_get(&mut driver, 0) })?;
        
        let mut device_count = 0;
        check(unsafe { shrimply_oneapi_device_count(driver, &mut device_count) })?;
        
        if device_count == 0 {
            return Err(DriverError(-1));
        }
        
        let mut device = std::ptr::null_mut();
        let device_idx = (ordinal % device_count as usize) as u32;
        check(unsafe { shrimply_oneapi_device_get(driver, device_idx, &mut device) })?;
        
        let mut context = std::ptr::null_mut();
        check(unsafe { shrimply_oneapi_context_create(&mut context, driver) })?;
        
        Ok(Arc::new(Self {
            driver,
            device,
            context,
            ordinal,
            queue_count: AtomicUsize::new(0),
        }))
    }

    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    pub fn device_uuid(&self) -> Result<[u8; DEVICE_UUID_BYTES], DriverError> {
        let mut uuid = [0; DEVICE_UUID_BYTES];
        check(unsafe { shrimply_oneapi_device_uuid(uuid.as_mut_ptr(), self.device) })?;
        Ok(uuid)
    }

    pub fn new_queue(self: &Arc<Self>) -> Result<Arc<OneApiQueue>, DriverError> {
        if self.queue_count.fetch_add(1, Ordering::Relaxed) == 0
            && self.synchronize().is_err()
        {
            self.queue_count.fetch_sub(1, Ordering::Relaxed);
            return Err(DriverError(-1));
        }
        
        let mut queue = std::ptr::null_mut();
        check(unsafe { 
            shrimply_oneapi_queue_create(&mut queue, self.context, self.device, 0) 
        })?;
        
        Ok(Arc::new(OneApiQueue {
            queue,
            device: self.device,
            context: Arc::clone(self),
        }))
    }

    pub fn default_queue(self: &Arc<Self>) -> Arc<OneApiQueue> {
        Arc::new(OneApiQueue {
            queue: std::ptr::null_mut(),
            device: self.device,
            context: self.clone(),
        })
    }

    pub fn synchronize(&self) -> Result<(), DriverError> {
        // Synchronize all queues - in real impl we'd track all queues
        Ok(())
    }
}

impl Drop for OneApiContext {
    fn drop(&mut self) {
        // Cleanup handled by Level Zero
    }
}

/// oneAPI queue, similar to CudaStream
#[derive(Debug)]
pub struct OneApiQueue {
    pub queue: *mut c_void,
    pub device: *mut c_void,
    pub context: Arc<OneApiContext>,
}

unsafe impl Send for OneApiQueue {}
unsafe impl Sync for OneApiQueue {}

impl OneApiQueue {
    pub fn new_cmdlist(self: &Arc<Self>) -> Result<OneApiCommandList, DriverError> {
        let mut cmdlist = std::ptr::null_mut();
        check(unsafe { 
            shrimply_oneapi_command_list_create(&mut cmdlist, self.context.context, self.device, self.queue) 
        })?;
        Ok(OneApiCommandList {
            cmdlist,
            queue: Arc::clone(self),
        })
    }

    pub fn execute(&self, cmdlists: &[OneApiCommandList]) -> Result<(), DriverError> {
        let mut ptrs: Vec<*mut c_void> = cmdlists.iter().map(|cl| cl.cmdlist).collect();
        check(unsafe { 
            shrimply_oneapi_queue_execute(self.queue, ptrs.len() as u32, ptrs.as_mut_ptr()) 
        })
    }

    pub fn synchronize(&self) -> Result<(), DriverError> {
        check(unsafe { shrimply_oneapi_queue_synchronize(self.queue) })
    }
}

/// oneAPI command list for recording operations
#[derive(Debug)]
pub struct OneApiCommandList {
    pub cmdlist: *mut c_void,
    pub queue: Arc<OneApiQueue>,
}

impl OneApiCommandList {
    pub fn close(&mut self) -> Result<(), DriverError> {
        check(unsafe { shrimply_oneapi_command_list_close(self.cmdlist) })
    }
}

impl Drop for OneApiCommandList {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Device buffer allocation
pub struct DeviceBuffer<T> {
    pointer: *mut c_void,
    length: usize,
    context: Arc<OneApiContext>,
    marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for DeviceBuffer<T> {}
unsafe impl<T: Send + Sync> Sync for DeviceBuffer<T> {}

impl<T> DeviceBuffer<T> {
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn context(&self) -> &Arc<OneApiContext> {
        &self.context
    }

    pub fn num_bytes(&self) -> usize {
        self.length
            .checked_mul(std::mem::size_of::<T>())
            .expect("oneAPI buffer size overflow")
    }

    pub fn zeroed(queue: &Arc<OneApiQueue>, length: usize) -> Result<Self, DriverError> {
        let bytes = length
            .checked_mul(std::mem::size_of::<T>())
            .expect("oneAPI buffer size overflow");
        let context = queue.context.clone();
        
        if bytes == 0 {
            return Ok(Self {
                pointer: std::ptr::null_mut(),
                length,
                context,
                marker: PhantomData,
            });
        }

        let mut pointer = std::ptr::null_mut();
        check(unsafe { 
            shrimply_oneapi_mem_alloc(&mut pointer, context.context, bytes) 
        })?;

        let buffer = Self {
            pointer,
            length,
            context,
            marker: PhantomData,
        };

        // Zero the memory
        let cmdlist = queue.new_cmdlist()?;
        check(unsafe { 
            shrimply_oneapi_memset_async(pointer, 0, bytes, cmdlist.cmdlist) 
        })?;
        queue.execute(&[cmdlist])?;
        queue.synchronize()?;

        Ok(buffer)
    }

    pub fn from_host(queue: &Arc<OneApiQueue>, data: &[T]) -> Result<Self, DriverError> {
        let mut result = Self::zeroed(queue, data.len())?;
        if result.is_empty() {
            return Ok(result);
        }
        result.copy_from_host(queue, data)?;
        Ok(result)
    }

    pub fn copy_from_host(&mut self, queue: &Arc<OneApiQueue>, data: &[T]) -> Result<(), DriverError> {
        assert_eq!(data.len(), self.length);
        if self.is_empty() {
            return Ok(());
        }

        let cmdlist = queue.new_cmdlist()?;
        check(unsafe { 
            shrimply_oneapi_memcpy_htod_async(
                self.pointer,
                data.as_ptr().cast(),
                self.num_bytes(),
                cmdlist.cmdlist,
            )
        })?;
        queue.execute(&[cmdlist])?;
        queue.synchronize()?;
        Ok(())
    }

    pub fn to_host_vec(&self, queue: &Arc<OneApiQueue>) -> Result<Vec<T>, DriverError> {
        if self.is_empty() {
            return Ok(Vec::new());
        }

        let mut output = Vec::<T>::with_capacity(self.length);
        let cmdlist = queue.new_cmdlist()?;
        
        check(unsafe { 
            shrimply_oneapi_memcpy_dtoh_async(
                output.as_mut_ptr().cast(),
                self.pointer,
                self.num_bytes(),
                cmdlist.cmdlist,
            )
        })?;
        
        queue.execute(&[cmdlist])?;
        queue.synchronize()?;
        unsafe { output.set_len(self.length) };
        Ok(output)
    }
}

impl<T> Drop for DeviceBuffer<T> {
    fn drop(&mut self) {
        if !self.pointer.is_null() {
            let _ = check(unsafe { shrimply_oneapi_mem_free(self.pointer) });
        }
    }
}

/// Kernel launch configuration
#[derive(Clone, Copy, Debug)]
pub struct LaunchConfig {
    pub grid_dim: (u32, u32, u32),
    pub block_dim: (u32, u32, u32),
    pub shared_mem_bytes: u32,
}

impl LaunchConfig {
    pub fn for_num_elems(n: u32) -> Self {
        const BLOCK: u32 = 256;
        Self {
            grid_dim: (n.div_ceil(BLOCK), 1, 1),
            block_dim: (BLOCK, 1, 1),
            shared_mem_bytes: 0,
        }
    }
}

/// Trait for types that can be copied to/from device
pub unsafe trait DeviceCopy: Copy {}

macro_rules! device_copy {
    ($($t:ty),*) => {
        $(unsafe impl DeviceCopy for $t {})*
    };
}

device_copy!(
    (),
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64
);

unsafe impl<T: DeviceCopy, const N: usize> DeviceCopy for [T; N] {}
unsafe impl<T: ?Sized> DeviceCopy for *const T {}
unsafe impl<T: ?Sized> DeviceCopy for *mut T {}

/// Launch a kernel with encoded arguments
pub unsafe fn launch_raw(
    kernel: &OneApiKernel,
    queue: &Arc<OneApiQueue>,
    config: LaunchConfig,
    mut arguments: Vec<*mut c_void>,
) -> Result<(), DriverError> {
    let (gx, gy, gz) = config.grid_dim;
    let (bx, by, bz) = config.block_dim;
    
    let cmdlist = queue.new_cmdlist()?;
    check(unsafe {
        shrimply_oneapi_kernel_launch(
            kernel.kernel,
            gx, gy, gz,
            bx, by, bz,
            config.shared_mem_bytes,
            cmdlist.cmdlist,
            arguments.as_mut_ptr(),
        )
    })?;
    queue.execute(&[cmdlist])?;
    Ok(())
}

/// oneAPI kernel
#[derive(Debug)]
pub struct OneApiKernel {
    pub kernel: *mut c_void,
    pub _module: Arc<OneApiModule>,
}

/// oneAPI module (contains kernels)
#[derive(Debug)]
pub struct OneApiModule {
    pub module: *mut c_void,
    pub context: Arc<OneApiContext>,
}

impl OneApiModule {
    pub fn load_from_spirv(
        context: &Arc<OneApiContext>,
        spirv: &[u8],
    ) -> Result<Arc<Self>, DriverError> {
        let mut module = std::ptr::null_mut();
        check(unsafe {
            shrimply_oneapi_module_create(
                &mut module,
                context.context,
                context.device,
                spirv.as_ptr().cast(),
                spirv.len(),
            )
        })?;
        Ok(Arc::new(Self {
            module,
            context: context.clone(),
        }))
    }

    pub fn get_kernel(self: &Arc<Self>, name: &str) -> Result<OneApiKernel, DriverError> {
        use std::ffi::CString;
        let name = CString::new(name).expect("Kernel name contains NUL");
        let mut kernel = std::ptr::null_mut();
        check(unsafe {
            shrimply_oneapi_module_get_kernel(&mut kernel, self.module, name.as_ptr())
        })?;
        Ok(OneApiKernel {
            kernel,
            _module: Arc::clone(self),
        })
    }
}

impl Drop for OneApiModule {
    fn drop(&mut self) {
        let _ = check(unsafe { shrimply_oneapi_module_destroy(self.module) });
    }
}

/// Event for synchronization
#[derive(Debug)]
pub struct OneApiEvent {
    pub event: *mut c_void,
    pub context: Arc<OneApiContext>,
}

impl OneApiEvent {
    pub fn record(&self, cmdlist: &OneApiCommandList) -> Result<(), DriverError> {
        check(unsafe { 
            shrimply_oneapi_event_record(self.event, cmdlist.cmdlist) 
        })
    }

    pub fn synchronize(&self) -> Result<(), DriverError> {
        check(unsafe { 
            shrimply_oneapi_event_synchronize(self.event) 
        })
    }

    pub fn elapsed_ms(&self, end: &Self) -> Result<f32, DriverError> {
        self.synchronize()?;
        end.synchronize()?;
        let mut value = 0.0;
        check(unsafe { 
            shrimply_oneapi_event_elapsed(&mut value, self.event, end.event) 
        })?;
        Ok(value)
    }
}

impl Drop for OneApiEvent {
    fn drop(&mut self) {
        let _ = check(unsafe { 
            shrimply_oneapi_event_destroy(self.event) 
        });
    }
}

#[macro_export]
macro_rules! __oneapi_arguments {
    ($arguments:ident, $launch:expr;) => { $launch };
    ($arguments:ident, $launch:expr; slice($value:expr) $(, $($rest:tt)*)?) => {{
        let value = $value;
        let mut pointer = value.pointer;
        let mut length = u64::try_from(value.len()).expect("oneAPI slice length exceeds u64");
        $arguments.push(::std::ptr::from_mut(&mut pointer).cast::<::std::ffi::c_void>());
        $arguments.push(::std::ptr::from_mut(&mut length).cast::<::std::ffi::c_void>());
        $crate::__oneapi_arguments!($arguments, $launch; $($($rest)*)?)
    }};
    ($arguments:ident, $launch:expr; slice_mut($value:expr) $(, $($rest:tt)*)?) => {{
        let value = $value;
        let mut pointer = value.pointer;
        let mut length = u64::try_from(value.len()).expect("oneAPI slice length exceeds u64");
        $arguments.push(::std::ptr::from_mut(&mut pointer).cast::<::std::ffi::c_void>());
        $arguments.push(::std::ptr::from_mut(&mut length).cast::<::std::ffi::c_void>());
        $crate::__oneapi_arguments!($arguments, $launch; $($($rest)*)?)
    }};
    ($arguments:ident, $launch:expr; $value:expr $(, $($rest:tt)*)?) => {{
        let mut value = $value;
        if ::std::mem::size_of_val(&value) != 0 {
            $arguments.push(::std::ptr::from_mut(&mut value).cast::<::std::ffi::c_void>());
        }
        $crate::__oneapi_arguments!($arguments, $launch; $($($rest)*)?)
    }};
}

#[macro_export]
macro_rules! oneapi_launch {
    (kernel:$kernel:path,queue:$queue:expr,config:$config:expr,args:[$($args:tt)*] $(,)?) => {{
        let mut arguments = Vec::<*mut ::std::ffi::c_void>::new();
        $crate::__oneapi_arguments!(
            arguments,
            $crate::launch_raw(&$kernel, $queue, $config, arguments),
            $($args)*
        )
    }};
}
