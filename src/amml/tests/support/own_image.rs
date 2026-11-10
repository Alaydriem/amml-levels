use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

// the address range of the running image: the test binary stands in for the mod DLL
pub struct OwnImage;

impl OwnImage {
    pub fn range() -> std::ops::Range<usize> {
        // SAFETY: a null name asks for the module of the running process
        let base = unsafe { GetModuleHandleW(std::ptr::null()) } as usize;
        // the PE header: e_lfanew at 0x3C, then SizeOfImage at 0x50 of the NT header
        // SAFETY: the base of a loaded image holds its DOS and NT headers
        let size = unsafe {
            let nt = base + (base as *const u32).byte_add(0x3C).read() as usize;
            ((nt + 0x50) as *const u32).read() as usize
        };
        base..base + size
    }
}
