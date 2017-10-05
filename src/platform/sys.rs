#![allow(non_snake_case)]

macro_rules! lib_extern {
    ( $($name:ident ( $($arg: ident : $argty: ty),* ) -> $retty: ty);+ ) => {
        $(pub unsafe fn $name($($arg: $argty),*) -> $retty {
            let func: Symbol<unsafe extern "stdcall" fn($($arg: $argty),*) -> $retty> =
                LIB.get(stringify!($name).as_bytes()).unwrap();
            func($($arg),*)
        })+
    };
}

pub mod bcp47langs {
    use winapi::winrt::hstring::HSTRING;
    use winapi::ctypes::{c_char, c_int};
    use winapi::um::winnt::WCHAR;
    use libloading::os::windows::*;

    lazy_static! {
        static ref LIB: Library = Library::new(r"C:\Windows\System32\BCP47Langs.dll").unwrap();
    }

    lib_extern! {
        GetUserLanguages(delimiter: c_char, string: *mut HSTRING) -> c_int;
        GetUserLanguageInputMethods(language: *const WCHAR, delimiter: c_char, string: *mut HSTRING) -> c_int;
        LcidFromBcp47(tag: HSTRING, lcid: *mut c_int) -> c_int;
        RemoveInputsForAllLanguagesInternal() -> c_int
    }
}

pub mod input {
    use winapi::ctypes::*;
    use winapi::um::winnt::WCHAR;
    use libloading::os::windows::*;

    lazy_static! {
        static ref LIB: Library = Library::new(r"C:\Windows\System32\input.dll").unwrap();
    }

    lib_extern! {
        InstallLayoutOrTip(tip_string: *const WCHAR, flags: c_int) -> c_int 
    }
}

pub mod winlangdb {
    use winapi::ctypes::*;
    use winapi::um::winnt::WCHAR;
    use winapi::winrt::hstring::HSTRING;
    use libloading::os::windows::*;

    lazy_static! {
        static ref LIB: Library = Library::new(r"C:\Windows\System32\winlangdb.dll").unwrap();
    }

    lib_extern! {
        EnsureLanguageProfileExists() -> c_int;
        GetLanguageNames(language: *const WCHAR, autonym: *mut WCHAR, english_name: *mut WCHAR, local_name: *mut WCHAR, script_name: *mut WCHAR) -> c_int;
        SetUserLanguages(delimiter: c_char, user_languages: HSTRING) -> c_int;
        GetDefaultInputMethodForLanguage(language: HSTRING, tip_string: *mut HSTRING) -> c_int;
        TransformInputMethodsForLanguage(tip_string: HSTRING, tag: HSTRING, transformed_tip_string: *mut HSTRING) -> c_int
    }
}
