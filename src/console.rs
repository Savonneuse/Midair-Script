#[cfg(windows)]
pub fn attach_to_parent() {
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
        SetStdHandle,
    };

    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return;
        }

        let mut console = None;
        for slot in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let current = GetStdHandle(slot);
            if !current.is_null() && current != INVALID_HANDLE_VALUE {
                continue;
            }
            let handle = *console.get_or_insert_with(|| {
                let name: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
                CreateFileW(
                    name.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    std::ptr::null_mut(),
                )
            });
            if handle != INVALID_HANDLE_VALUE {
                SetStdHandle(slot, handle);
            }
        }
    }
}

#[cfg(not(windows))]
pub fn attach_to_parent() {}
