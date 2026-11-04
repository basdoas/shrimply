#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use std::ffi::c_void;

// oneAPI SYCL types (simplified for Shrimply's needs)
pub type PIResult = i32;
pub type PIDevice = *mut c_void;
pub type PIContext = *mut c_void;
pub type PIQueue = *mut c_void;
pub type PIMem = *mut c_void;
pub type PIProgram = *mut c_void;
pub type PIKernel = *mut c_void;
pub type PIEvent = *mut c_void;

// Success code
pub const PI_SUCCESS: PIResult = 0;

// Error codes (subset)
pub const PI_ERROR_INVALID_VALUE: PIResult = -1;
pub const PI_ERROR_OUT_OF_RESOURCES: PIResult = -2;
pub const PI_ERROR_OUT_OF_HOST_MEMORY: PIResult = -3;

// Memory flags
pub const PI_MEM_FLAGS_READ_WRITE: u32 = 1;
pub const PI_MEM_FLAGS_HOST_ACCESS: u32 = 2;

// Queue flags
pub const PI_QUEUE_FLAG_OUT_OF_ORDER_EXEC_MODE_ENABLE: u32 = 1;
pub const PI_QUEUE_FLAG_PROFILING_ENABLE: u32 = 2;
