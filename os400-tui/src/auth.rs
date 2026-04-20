use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

pub const DEFAULT_SIGNON_USER: &str = "QSECOFR";
pub const DEFAULT_SIGNON_PASSWORD: &str = "l400";

const PAM_SUCCESS: c_int = 0;
const PAM_PROMPT_ECHO_OFF: c_int = 1;
const PAM_PROMPT_ECHO_ON: c_int = 2;
const PAM_ERROR_MSG: c_int = 3;
const PAM_TEXT_INFO: c_int = 4;

#[repr(C)]
struct PamHandle {
    _private: [u8; 0],
}

#[repr(C)]
struct PamMessage {
    msg_style: c_int,
    msg: *const c_char,
}

#[repr(C)]
struct PamResponse {
    resp: *mut c_char,
    resp_retcode: c_int,
}

#[repr(C)]
struct PamConv {
    conv: Option<
        unsafe extern "C" fn(
            num_msg: c_int,
            msg: *mut *const PamMessage,
            resp: *mut *mut PamResponse,
            appdata_ptr: *mut c_void,
        ) -> c_int,
    >,
    appdata_ptr: *mut c_void,
}

struct PamCredentials {
    username: CString,
    password: CString,
}

type PamStartFn = unsafe extern "C" fn(
    service_name: *const c_char,
    user: *const c_char,
    pam_conversation: *const PamConv,
    pamh: *mut *mut PamHandle,
) -> c_int;
type PamEndFn = unsafe extern "C" fn(pamh: *mut PamHandle, pam_status: c_int) -> c_int;
type PamAuthenticateFn = unsafe extern "C" fn(pamh: *mut PamHandle, flags: c_int) -> c_int;
type PamAcctMgmtFn = unsafe extern "C" fn(pamh: *mut PamHandle, flags: c_int) -> c_int;
type PamStrerrorFn =
    unsafe extern "C" fn(pamh: *mut PamHandle, errnum: c_int) -> *const c_char;

struct PamFns {
    _handle: *mut c_void,
    pam_start: PamStartFn,
    pam_end: PamEndFn,
    pam_authenticate: PamAuthenticateFn,
    pam_acct_mgmt: PamAcctMgmtFn,
    pam_strerror: PamStrerrorFn,
}

impl Drop for PamFns {
    fn drop(&mut self) {
        unsafe {
            libc::dlclose(self._handle);
        }
    }
}

pub fn authenticate_linux_user(username: &str, password: &str) -> Result<(), String> {
    let normalized = username.trim().to_lowercase();
    if normalized.is_empty() {
        return Err("Enter a user profile.".to_string());
    }

    if normalized == "root" {
        return Err("Profile ROOT is not available on Linux/400.".to_string());
    }

    let credentials = PamCredentials {
        username: CString::new(normalized.as_str())
            .map_err(|_| "User profile contains unsupported characters.".to_string())?,
        password: CString::new(password)
            .map_err(|_| "Password contains unsupported characters.".to_string())?,
    };
    let service = CString::new("login")
        .map_err(|_| "Unable to initialize Linux authentication.".to_string())?;
    let pam = load_pam().map_err(|_| {
        "Linux authentication is not available in this build/runtime.".to_string()
    })?;

    let conversation = PamConv {
        conv: Some(pam_conversation),
        appdata_ptr: (&credentials as *const PamCredentials).cast_mut().cast(),
    };

    let mut handle: *mut PamHandle = ptr::null_mut();
    let start_status = unsafe {
        (pam.pam_start)(
            service.as_ptr(),
            credentials.username.as_ptr(),
            &conversation,
            &mut handle,
        )
    };
    if start_status != PAM_SUCCESS {
        return Err(format_pam_error(&pam, handle, start_status));
    }

    let auth_status = unsafe { (pam.pam_authenticate)(handle, 0) };
    if auth_status != PAM_SUCCESS {
        unsafe {
            (pam.pam_end)(handle, auth_status);
        }
        return Err("User or password not correct.".to_string());
    }

    let acct_status = unsafe { (pam.pam_acct_mgmt)(handle, 0) };
    let end_status = unsafe { (pam.pam_end)(handle, acct_status) };
    if acct_status != PAM_SUCCESS {
        return Err(format_pam_error(&pam, ptr::null_mut(), acct_status));
    }
    if end_status != PAM_SUCCESS {
        return Err(format_pam_error(&pam, ptr::null_mut(), end_status));
    }

    Ok(())
}

unsafe extern "C" fn pam_conversation(
    num_msg: c_int,
    msg: *mut *const PamMessage,
    resp: *mut *mut PamResponse,
    appdata_ptr: *mut c_void,
) -> c_int {
    if num_msg <= 0 || msg.is_null() || resp.is_null() || appdata_ptr.is_null() {
        return -1;
    }

    let credentials = unsafe { &*(appdata_ptr as *const PamCredentials) };
    let responses = unsafe {
        libc::calloc(num_msg as usize, std::mem::size_of::<PamResponse>()) as *mut PamResponse
    };
    if responses.is_null() {
        return -1;
    }

    for idx in 0..num_msg as usize {
        let message = unsafe { *msg.add(idx) };
        if message.is_null() {
            free_pam_responses(responses, idx);
            return -1;
        }

        let response = unsafe { responses.add(idx) };
        let source = match unsafe { (*message).msg_style } {
            PAM_PROMPT_ECHO_ON => credentials.username.as_ptr(),
            PAM_PROMPT_ECHO_OFF => credentials.password.as_ptr(),
            PAM_ERROR_MSG | PAM_TEXT_INFO => ptr::null(),
            _ => {
                free_pam_responses(responses, idx);
                return -1;
            }
        };

        if !source.is_null() {
            let duplicated = unsafe { libc::strdup(source) };
            if duplicated.is_null() {
                free_pam_responses(responses, idx);
                return -1;
            }
            unsafe {
                (*response).resp = duplicated;
            }
        }
    }

    unsafe {
        *resp = responses;
    }
    PAM_SUCCESS
}

fn free_pam_responses(responses: *mut PamResponse, initialized: usize) {
    for idx in 0..initialized {
        let response = unsafe { responses.add(idx) };
        let ptr = unsafe { (*response).resp };
        if !ptr.is_null() {
            unsafe {
                libc::free(ptr.cast());
            }
        }
    }
    unsafe {
        libc::free(responses.cast());
    }
}

fn load_pam() -> Result<PamFns, ()> {
    let candidates = ["libpam.so.0", "libpam.so"];
    for candidate in candidates {
        let name = CString::new(candidate).map_err(|_| ())?;
        let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
        if handle.is_null() {
            continue;
        }

        let fns = unsafe {
            let pam_start = load_symbol::<PamStartFn>(handle, b"pam_start\0")?;
            let pam_end = load_symbol::<PamEndFn>(handle, b"pam_end\0")?;
            let pam_authenticate = load_symbol::<PamAuthenticateFn>(handle, b"pam_authenticate\0")?;
            let pam_acct_mgmt = load_symbol::<PamAcctMgmtFn>(handle, b"pam_acct_mgmt\0")?;
            let pam_strerror = load_symbol::<PamStrerrorFn>(handle, b"pam_strerror\0")?;
            PamFns {
                _handle: handle,
                pam_start,
                pam_end,
                pam_authenticate,
                pam_acct_mgmt,
                pam_strerror,
            }
        };
        return Ok(fns);
    }

    Err(())
}

unsafe fn load_symbol<T>(handle: *mut c_void, symbol: &[u8]) -> Result<T, ()>
where
    T: Copy,
{
    let ptr = unsafe { libc::dlsym(handle, symbol.as_ptr().cast()) };
    if ptr.is_null() {
        unsafe {
            libc::dlclose(handle);
        }
        return Err(());
    }

    Ok(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&ptr) })
}

fn format_pam_error(pam: &PamFns, handle: *mut PamHandle, status: c_int) -> String {
    let message = unsafe { (pam.pam_strerror)(handle, status) };
    if message.is_null() {
        "Linux authentication failed.".to_string()
    } else {
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    }
}
