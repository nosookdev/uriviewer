pub mod app;
pub mod loader;
pub mod nav;
pub mod types;
pub mod shell_ext;
pub mod tray;
pub mod hotkeys;
pub mod capture;
pub mod win_utils;

use crate::shell_ext::ShellExtension;
use std::ffi::c_void;
use std::ptr;
use windows::core::{implement, IUnknown, Interface, Result, GUID, HRESULT};
use windows::Win32::Foundation::{BOOL, CLASS_E_CLASSNOTAVAILABLE, E_FAIL, HINSTANCE, S_FALSE};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};

// CLSID for RustView Shell Extension: {D8E26C78-5B7E-4E38-9B7E-4E389B7E4E38}
pub const CLSID_RUSTVIEW_SHELL: GUID = GUID::from_u128(0xD8E26C78_5B7E_4E38_9B7E_4E389B7E4E38);

static mut DLL_INSTANCE: HINSTANCE = HINSTANCE(ptr::null_mut());

#[no_mangle]
pub unsafe extern "system" fn DllMain(
    hinst: HINSTANCE,
    reason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    if reason == 1 {
        // DLL_PROCESS_ATTACH
        DLL_INSTANCE = hinst;
    }
    BOOL::from(true)
}

#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_FAIL;
    }

    if *rclsid != CLSID_RUSTVIEW_SHELL {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    let factory: IClassFactory = ShellExtensionFactory.into();
    unsafe { factory.query(riid, ppv as *mut _) }
}

#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

#[implement(IClassFactory)]
struct ShellExtensionFactory;

impl IClassFactory_Impl for ShellExtensionFactory_Impl {
    fn CreateInstance(
        &self,
        _punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        if ppvobject.is_null() {
            return Err(E_FAIL.into());
        }

        let ext: windows::Win32::UI::Shell::IContextMenu = ShellExtension::new().into();
        unsafe { HRESULT::from(ext.query(riid, ppvobject as *mut _)).ok() }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}
