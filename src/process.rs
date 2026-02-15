use std::thread;
use std::time::Duration;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::ProcessStatus::EnumProcesses;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_READ,
};

pub struct ProcessManager;

impl ProcessManager {
    pub fn is_valorant_running() -> bool {
        Self::find_process("VALORANT-Win64-Shipping.exe").is_some()
    }

    pub fn wait_for_valorant_start() {
        while !Self::is_valorant_running() {
            thread::sleep(Duration::from_secs(2));
        }
    }

    pub fn wait_for_valorant_exit() {
        while Self::is_valorant_running() {
            thread::sleep(Duration::from_secs(2));
        }
    }

    fn find_process(name: &str) -> Option<u32> {
        unsafe {
            let mut processes = [0u32; 1024];
            let mut bytes_returned = 0u32;

            if EnumProcesses(
                processes.as_mut_ptr(),
                (processes.len() * std::mem::size_of::<u32>()) as u32,
                &mut bytes_returned,
            )
            .is_err()
            {
                return None;
            }

            let count = bytes_returned as usize / std::mem::size_of::<u32>();

            for &pid in &processes[..count] {
                if pid == 0 {
                    continue;
                }

                if let Some(process_name) = Self::get_process_name(pid) {
                    if process_name.eq_ignore_ascii_case(name) {
                        return Some(pid);
                    }
                }
            }

            None
        }
    }

    fn get_process_name(pid: u32) -> Option<String> {
        unsafe {
            let process_handle: HANDLE =
                match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
                    Ok(handle) => handle,
                    Err(_) => return None,
                };

            let mut buffer = vec![0u16; 260];
            let mut size = buffer.len() as u32;

            let result = QueryFullProcessImageNameW(
                process_handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            );

            let _ = CloseHandle(process_handle);

            if result.is_ok() && size > 0 {
                let path = String::from_utf16_lossy(&buffer[..size as usize]);
                path.split('\\').last().map(|s| s.to_string())
            } else {
                None
            }
        }
    }
}
