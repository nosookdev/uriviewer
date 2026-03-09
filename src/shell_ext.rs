use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr;
use windows::core::{implement, Result};
use windows::Win32::Foundation::{E_FAIL, HWND, MAX_PATH};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, FillRect, GetDC, GetSysColorBrush,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    SRCCOPY,
};
use windows::Win32::System::Com::{IDataObject, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::Registry::HKEY;
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT};

use std::fs::OpenOptions;
use std::io::Write;
use windows::Win32::UI::Shell::{
    DragQueryFileW, IContextMenu2_Impl, IContextMenu3_Impl, IContextMenu_Impl, IShellExtInit_Impl,
    CMF_DEFAULTONLY, CMF_VERBSONLY, CMINVOKECOMMANDINFO, HDROP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    InsertMenuItemW, HMENU, MENUITEMINFOW, MFS_DEFAULT, MFT_OWNERDRAW, MFT_SEPARATOR, MIIM_DATA,
    MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING,
};

fn log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("C:\\_new_ai_project\\uriviewer\\rustview.log")
    {
        let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), msg);
    }
}

fn get_viewer_executable_path() -> PathBuf {
    // DLL이 위치한 폴더에서 rustview.exe를 찾습니다.
    unsafe {
        let mut buffer = [0u16; MAX_PATH as usize];
        let h_inst = crate::DLL_INSTANCE;
        let len = windows::Win32::System::LibraryLoader::GetModuleFileNameW(h_inst, &mut buffer);
        if len > 0 {
            let dll_path = PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize]));
            let exe_path = dll_path.with_file_name("rustview.exe");
            if exe_path.exists() {
                return exe_path;
            }
        }
    }
    // 기본값으로 현재 디렉토리의 rustview.exe 반환
    PathBuf::from("C:\\_new_ai_project\\uriviewer\\target\\debug\\rustview.exe")
}

const WM_MEASUREITEM: u32 = 0x002C;
const WM_DRAWITEM: u32 = 0x002B;
const ODT_MENU: u32 = 1;
const COLOR_MENU: i32 = 4;

#[implement(
    windows::Win32::UI::Shell::IContextMenu,
    windows::Win32::UI::Shell::IContextMenu2,
    windows::Win32::UI::Shell::IContextMenu3,
    windows::Win32::UI::Shell::IShellExtInit
)]
pub struct ShellExtension {
    selected_files: std::sync::RwLock<Vec<PathBuf>>,
    preview_bitmap: std::sync::RwLock<Option<HBITMAP>>,
}

impl ShellExtension {
    pub fn new() -> Self {
        Self {
            selected_files: std::sync::RwLock::new(Vec::new()),
            preview_bitmap: std::sync::RwLock::new(None),
        }
    }
}

impl IShellExtInit_Impl for ShellExtension_Impl {
    fn Initialize(
        &self,
        _pidlfolder: *const windows::Win32::UI::Shell::Common::ITEMIDLIST,
        pdtobj: Option<&IDataObject>,
        _hkeyprogid: HKEY,
    ) -> Result<()> {
        log("Initialize called");
        let Some(data_object) = pdtobj else {
            log("No data object in Initialize");
            return Err(E_FAIL.into());
        };

        unsafe {
            let format_etc = FORMATETC {
                cfFormat: 15, // CF_HDROP
                ptd: ptr::null_mut(),
                dwAspect: 1, // DVASPECT_CONTENT
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
            };

            let medium = data_object.GetData(&format_etc)?;

            let hdrop = HDROP(medium.u.hGlobal.0 as *mut c_void);
            let count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
            let mut files = Vec::new();

            for i in 0..count {
                let mut buffer = [0u16; MAX_PATH as usize];
                let len = DragQueryFileW(hdrop, i, Some(&mut buffer));
                if len > 0 {
                    let path = String::from_utf16_lossy(&buffer[..len as usize]);
                    files.push(PathBuf::from(path));
                }
            }

            let mut locked_files = self.selected_files.write().unwrap();
            *locked_files = files;
        }

        Ok(())
    }
}

impl IContextMenu_Impl for ShellExtension_Impl {
    fn QueryContextMenu(
        &self,
        hmenu: HMENU,
        indexmenu: u32,
        idcmdfirst: u32,
        _idcmdlast: u32,
        _uflags: u32,
    ) -> windows::core::Result<()> {
        log(&format!("QueryContextMenu: idcmdfirst={}, indexmenu={}, uflags=0x{:X}", idcmdfirst, indexmenu, _uflags));
        let files = self.selected_files.read().unwrap();
        if files.is_empty() {
            log("QueryContextMenu: No files selected.");
            return Ok(());
        }

        let mut menu_index = indexmenu;
        let mut items_added = 0;

        unsafe {
            // 0. Top Separator
            let mii_top_sep = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_FTYPE,
                fType: MFT_SEPARATOR,
                ..Default::default()
            };
            let _ = InsertMenuItemW(hmenu, menu_index, true, &mii_top_sep);
            menu_index += 1;

            // 1. 우리뷰어로 변환
            let convert_text: Vec<u16> = "우리뷰어로 변환(&C)"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mii_conv = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_STRING | MIIM_ID,
                wID: idcmdfirst + items_added,
                dwTypeData: windows::core::PWSTR(convert_text.as_ptr() as *mut _),
                ..Default::default()
            };
            if InsertMenuItemW(hmenu, menu_index, true, &mii_conv).is_ok() {
                log("Insert CONVERT: Success");
                menu_index += 1;
                items_added += 1;
            }

            // 2. 우리뷰어로 보기
            let view_text: Vec<u16> = "우리뷰어로 보기(&V)"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mii_view = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_STRING | MIIM_ID | MIIM_STATE,
                wID: idcmdfirst + items_added,
                fState: MFS_DEFAULT, // 기본 항목으로 강조
                dwTypeData: windows::core::PWSTR(view_text.as_ptr() as *mut _),
                ..Default::default()
            };
            if InsertMenuItemW(hmenu, menu_index, true, &mii_view).is_ok() {
                log("Insert VIEW: Success");
                menu_index += 1;
                items_added += 1;
            }

            // 3. Preview item (Owner Draw) - 우리 메뉴 블록의 하단에 배치
            if let Some(first_file) = files.first() {
                log(&format!("Creating preview for: {:?}", first_file));
                if let Ok(hbitmap) = create_preview_bitmap(first_file) {
                    let mut cached = self.preview_bitmap.write().unwrap();
                    *cached = Some(hbitmap);

                    let mii = MENUITEMINFOW {
                        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                        fMask: MIIM_FTYPE | MIIM_ID | MIIM_DATA,
                        fType: MFT_OWNERDRAW,
                        wID: idcmdfirst + items_added,
                        dwItemData: 0xDEADBEEF,
                        ..Default::default()
                    };
                    if InsertMenuItemW(hmenu, menu_index, true, &mii).is_ok() {
                        log(&format!("Insert PREVIEW: Success (hbitmap={:?})", hbitmap));
                        menu_index += 1;
                        items_added += 1;
                    }

                    // 4. Bottom Separator
                    let mii_bot_sep = MENUITEMINFOW {
                        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                        fMask: MIIM_FTYPE,
                        fType: MFT_SEPARATOR,
                        ..Default::default()
                    };
                    let _ = InsertMenuItemW(hmenu, menu_index, true, &mii_bot_sep);
                    menu_index += 1;
                } else {
                    log("Failed to create preview bitmap");
                }
            }
        }

        // 중요: 최신 windows 크레이트에서 Result<()>를 반환할 때 항목 수(HRESULT)를 전달하기 위해
        // 성공 코드를 Err variant에 담아서 반환하는 트릭을 사용합니다.
        // HRESULT의 severity bit가 0이면 셸은 성공으로 인식하고 code 부분의 값을 항목 수로 사용합니다.
        log(&format!("QueryContextMenu returning items_added={}", items_added));
        Err(windows::core::Error::from_hresult(windows::core::HRESULT(items_added as i32)))
    }

    fn InvokeCommand(&self, pici: *const CMINVOKECOMMANDINFO) -> windows::core::Result<()> {
        let ici = unsafe { &*pici };
        let id = (ici.lpVerb.0 as usize) & 0xFFFF;
        log(&format!("InvokeCommand: id={}", id));

        let files = self.selected_files.read().unwrap();
        if files.is_empty() {
            log("InvokeCommand: No files selected");
            return Ok(());
        }

        let path = &files[0];
        let exe_path = get_viewer_executable_path();
        log(&format!("Executing: {:?} with arg {:?}", exe_path, path));

        match id {
            0 => {
                // Convert
                log("Convert command");
                std::process::Command::new(exe_path)
                    .arg("--convert")
                    .arg(path)
                    .spawn()
                    .map_err(|e| {
                        log(&format!("Failed to spawn: {}", e));
                        E_FAIL
                    })?;
            }
            1 | 2 => {
                // View (1) or Preview (2) clicked
                log(if id == 1 { "View command" } else { "Preview clicked (opening viewer)" });
                std::process::Command::new(exe_path).arg(path).spawn().map_err(|e| {
                    log(&format!("Failed to spawn: {}", e));
                    E_FAIL
                })?;
            }
            _ => {
                log(&format!("Unknown command ID: {}", id));
            }
        }

        Ok(())
    }

    fn GetCommandString(
        &self,
        _idcmd: usize,
        _utype: u32,
        _pwreserved: *const u32,
        _pszname: windows::core::PSTR,
        _cchmax: u32,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

impl IContextMenu2_Impl for ShellExtension_Impl {
    fn HandleMenuMsg(
        &self,
        umsg: u32,
        wparam: windows::Win32::Foundation::WPARAM,
        lparam: windows::Win32::Foundation::LPARAM,
    ) -> windows::core::Result<()> {
        let mut _lresult = windows::Win32::Foundation::LRESULT(0);
        self.HandleMenuMsg2(umsg, wparam, lparam, &mut _lresult)
    }
}

impl IContextMenu3_Impl for ShellExtension_Impl {
    fn HandleMenuMsg2(
        &self,
        umsg: u32,
        wparam: windows::Win32::Foundation::WPARAM,
        lparam: windows::Win32::Foundation::LPARAM,
        plresult: *mut windows::Win32::Foundation::LRESULT,
    ) -> windows::core::Result<()> {
        // 상세 로그
        if umsg == WM_MEASUREITEM || umsg == WM_DRAWITEM {
            log(&format!("HandleMenuMsg2: msg=0x{:X}, wparam=0x{:X}, lparam=0x{:X}", umsg, wparam.0, lparam.0));
        }

        match umsg {
            WM_MEASUREITEM => unsafe {
                let pmis = &mut *(lparam.0 as *mut MEASUREITEMSTRUCT);
                if pmis.CtlType.0 as u32 == ODT_MENU && pmis.itemData == 0xDEADBEEF {
                    pmis.itemWidth = 340;
                    // 기본 높이
                    let mut height = 20u32;
                    if let Some(hbitmap) = *self.preview_bitmap.read().unwrap() {
                        let mut bm = windows::Win32::Graphics::Gdi::BITMAP::default();
                        let _ = windows::Win32::Graphics::Gdi::GetObjectW(
                            hbitmap,
                            std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAP>() as i32,
                            Some(&mut bm as *mut _ as *mut _),
                        );
                        height = (bm.bmHeight + 12) as u32; // 이미지 높이 + 하단 여백
                    }
                    pmis.itemHeight = height;

                    if !plresult.is_null() {
                        *plresult = windows::Win32::Foundation::LRESULT(1);
                    }
                    return Ok(());
                }
            },
            WM_DRAWITEM => unsafe {
                let pdis = &*(lparam.0 as *const DRAWITEMSTRUCT);
                if pdis.CtlType.0 as u32 == ODT_MENU && pdis.itemData == 0xDEADBEEF {
                    log("HandleMenuMsg2: Drawing preview item...");
                    if let Some(hbitmap) = *self.preview_bitmap.read().unwrap() {
                        let hdc_mem = CreateCompatibleDC(pdis.hDC);
                        let old_obj = SelectObject(hdc_mem, hbitmap);

                        let mut bm = windows::Win32::Graphics::Gdi::BITMAP::default();
                        windows::Win32::Graphics::Gdi::GetObjectW(
                            hbitmap,
                            std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAP>() as i32,
                            Some(&mut bm as *mut _ as *mut _),
                        );

                        let x = (pdis.rcItem.left + pdis.rcItem.right - bm.bmWidth) / 2;
                        let y = pdis.rcItem.top; // 상단 밀착

                        let hbrush =
                            GetSysColorBrush(windows::Win32::Graphics::Gdi::SYS_COLOR_INDEX(4));
                        FillRect(pdis.hDC, &pdis.rcItem, hbrush);

                        let _ = BitBlt(
                            pdis.hDC,
                            x,
                            y,
                            bm.bmWidth,
                            bm.bmHeight,
                            hdc_mem,
                            0,
                            0,
                            SRCCOPY,
                        );

                        SelectObject(hdc_mem, old_obj);
                        let _ = DeleteDC(hdc_mem);
                    }
                    if !plresult.is_null() {
                        *plresult = windows::Win32::Foundation::LRESULT(1);
                    }
                    return Ok(());
                }
            },
            _ => {}
        }
        Ok(())
    }
}

unsafe fn create_preview_bitmap(path: &Path) -> Result<HBITMAP> {
    let img = image::open(path).map_err(|_| windows::core::Error::from_win32())?;
    // 썸네일 크기를 적절히 조정 (메뉴에 너무 크지 않게)
    let thumb = img.thumbnail(160, 120);
    let rgba = thumb.to_rgba8();
    let (w, h) = rgba.dimensions();

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32,
            biHeight: -(h as i32), // Top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let hdc = GetDC(HWND(ptr::null_mut()));
    let mut bits: *mut c_void = ptr::null_mut();
    let hbitmap = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)?;
    ReleaseDC(HWND(ptr::null_mut()), hdc);

    if !bits.is_null() {
        let bits_slice = std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize);
        for i in 0..(w * h) as usize {
            let r = rgba.as_raw()[i * 4];
            let g = rgba.as_raw()[i * 4 + 1];
            let b = rgba.as_raw()[i * 4 + 2];
            let a = rgba.as_raw()[i * 4 + 3];

            // 윈도우 메뉴는 프리멀티플라이드 알파(Pre-multiplied Alpha)를 기대하는 경우가 많습니다.
            let alpha_f = a as f32 / 255.0;
            bits_slice[i * 4] = (b as f32 * alpha_f) as u8; // B
            bits_slice[i * 4 + 1] = (g as f32 * alpha_f) as u8; // G
            bits_slice[i * 4 + 2] = (r as f32 * alpha_f) as u8; // R
            bits_slice[i * 4 + 3] = a; // A
        }
    }

    Ok(hbitmap)
}
