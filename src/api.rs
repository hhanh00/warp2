use std::ffi::{c_char, CStr};
use allo_isolate::ffi;
use zcash_primitives::consensus::Network::MainNetwork;

pub static mut POST_COBJ: Option<ffi::DartPostCObjectFnType> = None;

#[no_mangle]
pub unsafe extern "C" fn dart_post_cobject(ptr: ffi::DartPostCObjectFnType) {
    POST_COBJ = Some(ptr);
}

#[tokio::main]
#[no_mangle]
pub async unsafe extern "C" fn full_scan(url: *mut c_char, fvk: *mut c_char, port: i64) -> u64 {
    let url = CStr::from_ptr(url).to_string_lossy();
    let fvk = CStr::from_ptr(fvk).to_string_lossy();
    crate::warp::scan::full_scan(&MainNetwork, &url, &fvk, port).await.unwrap()
}
