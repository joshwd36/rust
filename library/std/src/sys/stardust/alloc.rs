use crate::alloc::{GlobalAlloc, Layout, System};

#[stable(feature = "alloc_system_type", since = "1.28.0")]
unsafe impl GlobalAlloc for System {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = libc::xmalloc_align(layout.size(), layout.align());
        ptr as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        libc::xfree(ptr as *const libc::c_void)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let ptr = libc::xrealloc(ptr as *mut libc::c_void, new_size, layout.align());
        ptr as *mut u8
    }
}
