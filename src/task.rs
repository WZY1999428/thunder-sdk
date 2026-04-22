include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
use std::os::raw::c_void;

#[derive(Debug)]
pub struct TaskInfo {
    pub url: String,
    pub save_name: String,
    pub save_path: String,
    pub traffic: TrafficInfo,
    pub creation_time: u64,
    pub completion_time: u64,
}

#[repr(C)] // 必须加这个，保证内存布局和 C 一致
#[derive(Debug, Default)]
pub struct TrafficInfo {
    pub origin_size: u64,
    pub p2p_size: u64,
    pub p2s_size: u64,
    pub dcdn_size: u64,
}

// 1. 定义转换逻辑
pub trait FromBuffer {
    fn from_buffer(buf: Vec<u8>) -> Self;
}

// 2. 为 String 实现转换
impl FromBuffer for String {
    fn from_buffer(buf: Vec<u8>) -> Self {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..end]).into_owned()
    }
}

// 3. 为 u64 实现转换（比如有些 ID 或 Size 是 8 字节）
impl FromBuffer for u64 {
    fn from_buffer(buf: Vec<u8>) -> Self {
        let mut bytes = [0u8; 8];
        let len = buf.len().min(8);
        bytes[..len].copy_from_slice(&buf[..len]);
        u64::from_le_bytes(bytes) // 假设 C 端是小端序
    }
}

impl FromBuffer for TrafficInfo {
    fn from_buffer(buf: Vec<u8>) -> Self {
        // 安全第一：检查长度是否匹配 xl_dl_task_traffic_info_t (32字节)
        let expected_size = std::mem::size_of::<xl_dl_task_traffic_info_t>();

        if buf.len() < expected_size {
            return Self {
                origin_size: 0,
                p2p_size: 0,
                p2s_size: 0,
                dcdn_size: 0,
            };
        }

        unsafe {
            // 将 buffer 中的数据视为 C 结构体并读取
            let c_info = *(buf.as_ptr() as *const xl_dl_task_traffic_info_t);

            Self {
                origin_size: c_info.origin_size,
                p2p_size: c_info.p2p_size,
                p2s_size: c_info.p2s_size,
                dcdn_size: c_info.dcdn_size,
            }
        }
    }
}

pub fn get_task_property<T: FromBuffer>(tid: u64, k: &str) -> Result<T, String> {
    let mut len = 0;
    let key = super::to_cstring(k)?;
    let ret = unsafe { xl_dl_get_task_info(tid, key.as_ptr(), std::ptr::null_mut(), &mut len) };
    if ret != 0 {
        return Err(format!(
            "设置最大同时进行的下载任务数失败：{}",
            super::codes::get_error_msg(ret)
        ));
    }
    if len == 0 {
        return Err("Length is 0".into());
    }

    let mut buffer = vec![0u8; len as usize];
    let ret = unsafe {
        xl_dl_get_task_info(
            tid,
            key.as_ptr(),
            buffer.as_mut_ptr() as *mut c_void,
            &mut len,
        )
    };

    if ret != 0 {
        return Err(format!("获取{}信息失败", k));
    }

    // 5. 调用 Trait 方法返回 T
    Ok(T::from_buffer(buffer))
}
