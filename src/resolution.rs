use std::mem;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsW, EnumDisplaySettingsW, DEVMODEW, CDS_FULLSCREEN, DISP_CHANGE_SUCCESSFUL,
    ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE, DM_PELSWIDTH, DM_PELSHEIGHT,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

pub struct ResolutionManager;

impl ResolutionManager {
    pub fn get_current() -> Option<Resolution> {
        unsafe {
            let mut devmode: DEVMODEW = mem::zeroed();
            devmode.dmSize = mem::size_of::<DEVMODEW>() as u16;

            if EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut devmode).as_bool() {
                Some(Resolution::new(
                    devmode.dmPelsWidth,
                    devmode.dmPelsHeight,
                ))
            } else {
                None
            }
        }
    }

    pub fn set_resolution(resolution: Resolution) -> bool {
        unsafe {
            let mut devmode: DEVMODEW = mem::zeroed();
            devmode.dmSize = mem::size_of::<DEVMODEW>() as u16;
            devmode.dmPelsWidth = resolution.width;
            devmode.dmPelsHeight = resolution.height;
            devmode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;

            ChangeDisplaySettingsW(Some(&devmode), CDS_FULLSCREEN) == DISP_CHANGE_SUCCESSFUL
        }
    }

    pub fn list_available() -> Vec<Resolution> {
        let mut resolutions = Vec::new();
        let mut index = 0;

        unsafe {
            loop {
                let mut devmode: DEVMODEW = mem::zeroed();
                devmode.dmSize = mem::size_of::<DEVMODEW>() as u16;

                if !EnumDisplaySettingsW(None, ENUM_DISPLAY_SETTINGS_MODE(index), &mut devmode).as_bool() {
                    break;
                }

                let res = Resolution::new(devmode.dmPelsWidth, devmode.dmPelsHeight);
                if !resolutions.contains(&res) && res.width >= 800 && res.height >= 600 {
                    resolutions.push(res);
                }

                index += 1;
            }
        }

        resolutions.sort_by(|a, b| {
            if a.width != b.width {
                a.width.cmp(&b.width)
            } else {
                a.height.cmp(&b.height)
            }
        });

        resolutions
    }
}