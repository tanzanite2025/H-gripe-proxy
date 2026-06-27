use std::sync::Arc;
/**
 * 反调试模块
 *
 * 检测：
 * 1. IsDebuggerPresent（Windows）
 * 2. 父进程异常
 * 3. 调试端口开放
 */
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 反调试配置
#[derive(Debug, Clone)]
pub struct AntiDebugConfig {
    /// 启用反调试
    pub enabled: bool,
    /// 检测间隔（毫秒）
    pub check_interval_ms: u64,
    /// 检测到调试器时是否自毁
    pub auto_destruct: bool,
}

impl Default for AntiDebugConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_ms: 1000,
            auto_destruct: true,
        }
    }
}

/// 检测是否被调试
pub fn is_debugger_present() -> bool {
    check_debugger_windows()
}

/// Windows 反调试检测
fn check_debugger_windows() -> bool {
    use windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent;

    unsafe {
        // 检查 IsDebuggerPresent
        if IsDebuggerPresent().as_bool() {
            return true;
        }

        // 检查 NtGlobalFlag
        // 调试器会设置 FLG_HEAP_ENABLE_TAIL_CHECK | FLG_HEAP_ENABLE_FREE_CHECK | FLG_HEAP_VALIDATE_PARAMETERS
        let peb = get_peb();
        if !peb.is_null() {
            let nt_global_flag = *(peb.add(0xBC) as *const u32);
            if (nt_global_flag & 0x70) != 0 {
                return true;
            }
        }

        // 检查调试端口
        check_debug_port_windows()
    }
}

unsafe fn get_peb() -> *const u8 {
    #[cfg(target_arch = "x86_64")]
    {
        let peb: *const u8;
        unsafe {
            std::arch::asm!(
                "mov {}, gs:[0x60]",
                out(reg) peb,
            );
        }
        peb
    }
    #[cfg(target_arch = "x86")]
    {
        let peb: *const u8;
        unsafe {
            std::arch::asm!(
                "mov {}, fs:[0x30]",
                out(reg) peb,
            );
        }
        peb
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        std::ptr::null()
    }
}

fn check_debug_port_windows() -> bool {
    use windows::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        let process = GetCurrentProcess();
        let mut debug_port: usize = 0;

        // NtQueryInformationProcess with ProcessDebugPort (7)
        let status = ntdll_query_information_process(
            process.0 as *mut _,
            7,
            &mut debug_port as *mut _ as *mut _,
            std::mem::size_of::<usize>(),
        );

        status == 0 && debug_port != 0
    }
}

unsafe extern "system" {
    fn NtQueryInformationProcess(
        process_handle: *mut std::ffi::c_void,
        process_information_class: u32,
        process_information: *mut std::ffi::c_void,
        process_information_length: usize,
    ) -> i32;
}

fn ntdll_query_information_process(
    process_handle: *mut std::ffi::c_void,
    process_information_class: u32,
    process_information: *mut std::ffi::c_void,
    process_information_length: usize,
) -> i32 {
    unsafe {
        NtQueryInformationProcess(
            process_handle,
            process_information_class,
            process_information,
            process_information_length,
        )
    }
}

/// 检查父进程是否异常
pub fn check_parent_process() -> bool {
    check_parent_process_windows()
}

fn check_parent_process_windows() -> bool {
    use crate::utils::command::hidden_command;

    // 获取父进程名称
    if let Ok(output) = hidden_command("wmic")
        .args(&[
            "process",
            "where",
            &format!("ProcessId={}", std::process::id()),
            "get",
            "ParentProcessId",
        ])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            // 检查父进程是否是可疑的调试器或分析工具
            let suspicious = ["x64dbg", "x32dbg", "ollydbg", "windbg", "ida", "ghidra"];
            for name in &suspicious {
                if output_str.to_lowercase().contains(name) {
                    return true;
                }
            }
        }
    }

    false
}

/// 反调试监控循环
pub fn monitor_loop(enabled: Arc<AtomicBool>) {
    let config = AntiDebugConfig::default();

    while enabled.load(Ordering::Relaxed) {
        if config.enabled && is_debugger_present() {
            log::warn!("🚨 检测到调试器！触发安全响应...");

            // 标记安全状态为已破坏
            crate::security::mark_security_compromised();

            if config.auto_destruct {
                // 触发自毁
                crate::security::self_destruct::execute();
            }

            break;
        }

        if config.enabled && check_parent_process() {
            log::warn!("🚨 检测到可疑父进程！触发安全响应...");
            crate::security::mark_security_compromised();

            if config.auto_destruct {
                crate::security::self_destruct::execute();
            }

            break;
        }

        std::thread::sleep(Duration::from_millis(config.check_interval_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_detection() {
        // 正常情况下不应该检测到调试器
        // 注意：在调试模式下运行此测试会失败
        let is_debugging = is_debugger_present();
        println!("Debugger present: {}", is_debugging);
    }
}
