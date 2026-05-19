extern "C" {
    pub fn host_log(level: i32, ptr: *const u8, len: i32);
    pub fn host_get_config(ptr: *mut u8, max_len: i32) -> i32;
}
