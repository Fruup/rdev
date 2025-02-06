use crate::rdev::{Event, EventType, ListenError};
use crate::windows::common::{
    convert, set_key_hook, set_mouse_hook, HookError, KEYBOARD, KEY_HOOK, MOUSE_HOOK,
};
use std::os::raw::c_int;
use std::time::SystemTime;
use winapi::shared::minwindef::{LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::HHOOK;
use winapi::um::winuser::{CallNextHookEx, HC_ACTION};

use super::common::{remove_key_hook, remove_mouse_hook};

static mut GLOBAL_CALLBACK: Option<Box<dyn FnMut(Event)>> = None;

impl From<HookError> for ListenError {
    fn from(error: HookError) -> Self {
        match error {
            HookError::Mouse(code) => ListenError::MouseHookError(code),
            HookError::Key(code) => ListenError::KeyHookError(code),
        }
    }
}

unsafe fn _raw_callback(hook: HHOOK, code: c_int, param: WPARAM, lpdata: LPARAM) -> LRESULT {
    if code == HC_ACTION {
        let opt = convert(param, lpdata);
        if let Some(event_type) = opt {
            let name = match &event_type {
                EventType::KeyPress(_key) => match (*KEYBOARD).lock() {
                    Ok(mut keyboard) => keyboard.get_name(lpdata),
                    Err(_) => None,
                },
                _ => None,
            };
            let event = Event {
                event_type,
                time: SystemTime::now(),
                name,
            };
            if let Some(callback) = &mut GLOBAL_CALLBACK {
                callback(event);
            }
        }
    }
    CallNextHookEx(hook, code, param, lpdata)
}

unsafe extern "system" fn raw_key_callback(code: c_int, param: WPARAM, lpdata: LPARAM) -> LRESULT {
    _raw_callback(KEY_HOOK, code, param, lpdata)
}

unsafe extern "system" fn raw_mouse_callback(
    code: c_int,
    param: WPARAM,
    lpdata: LPARAM,
) -> LRESULT {
    _raw_callback(MOUSE_HOOK, code, param, lpdata)
}

pub fn listen<T>(callback: T) -> Result<impl Fn(), ListenError>
where
    T: FnMut(Event) + 'static,
{
    unsafe {
        GLOBAL_CALLBACK = Some(Box::new(callback));
        set_key_hook(raw_key_callback)?;
        set_mouse_hook(raw_mouse_callback)?;

        // The following is a blocking call. It will block the current thread until a message is received.
        // I found that it's not necessary, so I removed it so the caller doesn't get blocked.

        // GetMessageA(null_mut(), null_mut(), 0, 0);

        let stop_fn = || {
            // TODO: lead these errors back to the caller
            let _ = remove_key_hook();
            let _ = remove_mouse_hook();
            GLOBAL_CALLBACK = None;
        };

        Ok(stop_fn)
    }
}
