pub(crate) fn aligned_p(ptr: *const u8) -> bool {
    (ptr as usize) % page_size() == 0
}

pub(crate) fn page_size() -> usize {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
}

pub(crate) fn bytes2pages(bytes: usize) -> usize {
    let page_size = page_size();
    (bytes + page_size - 1) / page_size
}
