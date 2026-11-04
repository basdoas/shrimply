//! Intel oneAPI Open Image Denoise (OIDN) backend for Shrimply
//!
//! This crate provides Intel GPU-accelerated denoising using OIDN,
//! which is part of the Intel oneAPI Rendering Toolkit.

use std::ffi::{c_char, c_int, c_void};
use std::ptr::{self, NonNull};
use std::sync::Arc;

use shrimply_gpu_oneapi::{OneApiContext, OneApiQueue};

const ERROR_CAPACITY: usize = 1024;

// Opaque handle for OIDN denoiser
enum NativeDenoiser {}

unsafe extern "C" {
    fn shrimply_oidn_denoiser_create(
        context: *mut c_void,
        queue: *mut c_void,
        width: u32,
        height: u32,
        output: *mut *mut NativeDenoiser,
        error: *mut c_char,
        error_capacity: usize,
    ) -> c_int;
    fn shrimply_oidn_denoiser_invoke(
        denoiser: *mut NativeDenoiser,
        queue: *mut c_void,
        beauty: *mut c_void,
        albedo: *mut c_void,
        normal: *mut c_void,
        output: *mut c_void,
        error: *mut c_char,
        error_capacity: usize,
    ) -> c_int;
    fn shrimply_oidn_denoiser_destroy(
        denoiser: *mut NativeDenoiser,
        error: *mut c_char,
        error_capacity: usize,
    ) -> c_int;
}

/// Intel oneAPI OIDN denoiser
pub struct OneApiDenoiser {
    native: NonNull<NativeDenoiser>,
    context: Arc<OneApiContext>,
    width: u32,
    height: u32,
}

/// Input buffers for denoising
pub struct DenoiseInputs<'a> {
    pub beauty: &'a [u8],
    pub albedo: Option<&'a [u8]>,
    pub normal: Option<&'a [u8]>,
}

impl OneApiDenoiser {
    /// Create a new OIDN denoiser
    pub fn new(
        context: Arc<OneApiContext>,
        queue: &OneApiQueue,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("OIDN denoiser dimensions must be nonzero".to_string());
        }
        
        let mut native = ptr::null_mut();
        ffi_result(|error, capacity| unsafe {
            shrimply_oidn_denoiser_create(
                context.context,
                queue.queue,
                width,
                height,
                &mut native,
                error,
                capacity,
            )
        })?;
        
        Ok(Self {
            native: NonNull::new(native).expect("successful OIDN creation returns a handle"),
            context,
            width,
            height,
        })
    }

    /// Denoise an image
    pub fn denoise<'a>(
        &mut self,
        queue: &OneApiQueue,
        inputs: DenoiseInputs<'a>,
        output: &mut [u8],
    ) -> Result<(), String> {
        ffi_result(|error, capacity| unsafe {
            // Note: In a real implementation, we'd need to handle the device pointers properly
            // For now, we pass null pointers as placeholders
            shrimply_oidn_denoiser_invoke(
                self.native.as_ptr(),
                queue.queue,
                inputs.beauty.as_ptr() as *mut c_void,
                inputs.albedo.map_or(std::ptr::null_mut(), |a| a.as_ptr() as *mut c_void),
                inputs.normal.map_or(std::ptr::null_mut(), |n| n.as_ptr() as *mut c_void),
                output.as_mut_ptr() as *mut c_void,
                error,
                capacity,
            )
        })
    }

    /// Check if the denoiser matches the given dimensions
    pub fn matches(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }
}

impl Drop for OneApiDenoiser {
    fn drop(&mut self) {
        if ffi_result(|error, capacity| unsafe {
            shrimply_oidn_denoiser_destroy(self.native.as_ptr(), error, capacity)
        }).is_err() {
            std::process::abort();
        }
    }
}

fn ffi_result(call: impl FnOnce(*mut c_char, usize) -> c_int) -> Result<(), String> {
    let mut error = [0_u8; ERROR_CAPACITY];
    if call(error.as_mut_ptr().cast(), error.len()) == 0 {
        return Ok(());
    }
    let end = error
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(error.len());
    Err(String::from_utf8_lossy(&error[..end]).into_owned())
}
