//! Everything (voidtools) DLL IPC integration: dynamic DLL load + serialized
//! query into (name, full_path, is_folder) results.
//!
//! Focused Everything integration re-exported by the platform facade.

type FnSetSearchW = unsafe extern "system" fn(*const u16);
type FnSetMax = unsafe extern "system" fn(u32);
type FnQueryW = unsafe extern "system" fn(i32) -> i32;
type FnGetNum = unsafe extern "system" fn() -> u32;
type FnGetFullPathW = unsafe extern "system" fn(u32, *mut u16, u32) -> u32;
type FnGetFileNameW = unsafe extern "system" fn(u32) -> *const u16;
type FnIsFolderResult = unsafe extern "system" fn(u32) -> i32;

struct EvFns {
    set_search_w: FnSetSearchW,
    set_max: FnSetMax,
    query_w: FnQueryW,
    get_num: FnGetNum,
    get_full_path_w: FnGetFullPathW,
    get_file_name_w: FnGetFileNameW,
    is_folder_result: FnIsFolderResult,
}

// fn ptrs are always Send+Sync
unsafe impl Send for EvFns {}
unsafe impl Sync for EvFns {}

static EV_FNS: std::sync::OnceLock<Option<EvFns>> = std::sync::OnceLock::new();
static EV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn init_everything() -> Option<EvFns> {
    use windows::core::PCWSTR;
    use windows::Win32::System::LibraryLoader::LoadLibraryW;

    let dll_paths: &[&str] = &[
        "Everything64.dll",
        r"C:\Program Files\Everything\Everything64.dll",
        r"C:\Program Files (x86)\Everything\Everything32.dll",
    ];

    for &path in dll_paths {
        let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let Ok(hlib) = (unsafe { LoadLibraryW(PCWSTR(wide.as_ptr())) }) else {
            continue;
        };
        if let Some(fns) = load_ev_fns(hlib) {
            return Some(fns);
        }
    }
    None
}

fn load_ev_fns(hlib: windows::Win32::Foundation::HMODULE) -> Option<EvFns> {
    use windows::core::PCSTR;
    use windows::Win32::System::LibraryLoader::GetProcAddress;

    unsafe fn gp<F: Copy>(hlib: windows::Win32::Foundation::HMODULE, name: &[u8]) -> Option<F> {
        let raw = GetProcAddress(hlib, PCSTR(name.as_ptr()))?;
        Some(std::mem::transmute_copy(&raw))
    }

    unsafe {
        Some(EvFns {
            set_search_w: gp(hlib, b"Everything_SetSearchW\0")?,
            set_max: gp(hlib, b"Everything_SetMax\0")?,
            query_w: gp(hlib, b"Everything_QueryW\0")?,
            get_num: gp(hlib, b"Everything_GetNumResults\0")?,
            get_full_path_w: gp(hlib, b"Everything_GetResultFullPathNameW\0")?,
            get_file_name_w: gp(hlib, b"Everything_GetResultFileNameW\0")?,
            is_folder_result: gp(hlib, b"Everything_IsFolderResult\0")?,
        })
    }
}

/// 偵測 Everything DLL 是否可載入（不啟動服務不行查詢）。
pub fn check_everything() -> bool {
    EV_FNS.get_or_init(init_everything).is_some()
}

/// 透過 Everything DLL IPC 查詢，回傳 (name, full_path, is_folder) 清單。
/// Everything 服務需正在執行，否則回傳空清單。
pub fn everything_search(query: &str, max_results: u32) -> Vec<(String, String, bool)> {
    let Some(fns) = EV_FNS.get_or_init(init_everything) else {
        return Vec::new();
    };

    // Everything DLL 內部使用全域狀態，必須序列化呼叫
    let _guard = EV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    unsafe {
        let q_wide: Vec<u16> = query.encode_utf16().chain(Some(0)).collect();
        (fns.set_search_w)(q_wide.as_ptr());
        (fns.set_max)(max_results);

        if (fns.query_w)(1) == 0 {
            return Vec::new();
        }

        let count = (fns.get_num)().min(max_results);
        let mut results = Vec::with_capacity(count as usize);

        for i in 0..count {
            let mut buf = vec![0u16; 4096];
            let len = (fns.get_full_path_w)(i, buf.as_mut_ptr(), buf.len() as u32);
            if len == 0 {
                continue;
            }
            let full_path = String::from_utf16_lossy(&buf[..len as usize]).to_string();
            if full_path.contains('\u{FFFD}') {
                eprintln!(
                    "[keynova] skipping Everything result with invalid UTF-16 path at index {i}"
                );
                continue;
            }

            let name_ptr = (fns.get_file_name_w)(i);
            if name_ptr.is_null() {
                continue;
            }
            let name = read_wstr(name_ptr);
            if name.contains('\u{FFFD}') {
                eprintln!(
                    "[keynova] skipping Everything result with invalid UTF-16 name at index {i}"
                );
                continue;
            }
            let is_folder = (fns.is_folder_result)(i) != 0;
            results.push((name, full_path, is_folder));
        }

        results
    }
}

unsafe fn read_wstr(ptr: *const u16) -> String {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len)).to_string()
}
