// the 16 bytes of a `std::string_view`: the data pointer, then the length. The game takes it by pointer to a copy
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StringView {
    pub data: *const u8,
    pub len: usize,
}

impl StringView {
    pub fn of(text: &str) -> Self {
        Self { data: text.as_ptr(), len: text.len() }
    }
}
