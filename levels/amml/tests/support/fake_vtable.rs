// a leaked table of code pointers, as the first qword of a fake C++ object points to; slot 0 is unused
pub struct FakeVtable;

impl FakeVtable {
    pub fn leak(slots: &[usize]) -> *const usize {
        let mut table = vec![0usize];
        table.extend_from_slice(slots);
        Box::leak(table.into_boxed_slice()).as_ptr()
    }
}
