include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

mod batch_task;
mod codes;
mod task;
mod token;
use std::collections::HashMap;
use std::env;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr::null_mut;
use token::build_token;

use batch_task::BatchInfo;
use codes::{get_error_msg, get_task_state_msg};
use task::{TaskInfo, TrafficInfo, get_task_property};

pub struct Thunder {
    pub app_id: String,
    pub app_version: String,
    pub cfg_path: String,
    pub save_tasks: u8,
    pub session_id: Option<String>,
}

impl Thunder {
    /// appid ; // 应用 ID,控制台创建应用时生成的 app_id
    /// app_version ; // 应用版本号
    /// cfg_path ; // 配置信息及任务信息保存路径, utf8 编码
    /// save_tasks ; // 是否保存任务信息，1：保存 0：不保存
    pub fn init(
        appid: &str,
        app_version: &str,
        cfg_path: &str,
        save_tasks: u8,
    ) -> Result<Self, String> {
        let ret = unsafe {
            let appid = to_cstring(appid)?;
            let app_version = to_cstring(app_version)?;
            let cfg_path = to_cstring(cfg_path)?;
            xl_dl_init(&xl_dl_init_param_t {
                app_id: appid.as_ptr(),
                app_version: app_version.as_ptr(),
                cfg_path: cfg_path.as_ptr(),
                save_tasks,
            })
        };
        if ret != 0 {
            return Err(format!("初始化失败： {}", get_error_msg(ret)));
        }
        Ok(Self {
            app_id: appid.to_string(),
            app_version: app_version.to_string(),
            cfg_path: cfg_path.to_string(),
            save_tasks: save_tasks,
            session_id: None,
        })
    }
    ///反初始化。
    pub fn uninit() -> Result<(), String> {
        unsafe {
            let ret = xl_dl_uninit();
            if ret != 0 {
                return Err(format!("返初始化失败： {}", get_error_msg(ret)));
            }
        }
        Ok(())
    }
    ///登录。
    /// app_id           (应用ID: app_id，控制台-应用管理页面获取)
    /// app_secret       String (应用密钥)
    /// issuer           (合作商标识，填合作商自己的域名或其它标识。例: "xxx.com")
    /// expire_seconds   (单位：秒,推荐设置：24小时)
    pub fn login(
        &mut self,
        app_id: &str,
        app_secret: &str,
        issuer: &str,
        expire_seconds: u64,
    ) -> Result<(), String> {
        let token = to_cstring(
            &build_token(app_id, app_secret, issuer, expire_seconds).map_err(|e| e.to_string())?,
        )?;
        let mut session_buf = vec![0u8; 4096];
        let ret = unsafe { xl_dl_login(token.as_ptr(), session_buf.as_mut_ptr() as *mut i8) };
        if ret != 0 {
            return Err(format!("login fial： {}", get_error_msg(ret)));
        }

        // 2️⃣ 找到 C 字符串结束位置
        let len = session_buf.iter().position(|&c| c == 0).unwrap_or(4096);

        let session_id = String::from_utf8_lossy(&session_buf[..len]).to_string();
        self.session_id = Some(session_id);
        Ok(())
    }
    /// 获取未完成任务 ID，包括已失败的任务。
    pub fn get_unfinished_tasks() -> Result<Vec<u64>, String> {
        let mut count = 0;
        let ret = unsafe { xl_dl_get_unfinished_tasks(null_mut(), &mut count) };
        if ret != 0 {
            return Err(format!("获取失败： {}", get_error_msg(ret)));
        }
        let mut task_id_array = vec![0u64; count as usize];
        let ret = unsafe { xl_dl_get_unfinished_tasks(task_id_array.as_mut_ptr(), &mut count) };
        if ret != 0 {
            return Err(format!("获取失败： {}", get_error_msg(ret)));
        }
        Ok(task_id_array)
    }
    /// 获取已完成任务 ID。
    pub fn get_finished_tasks() -> Result<Vec<u64>, String> {
        let mut count = 0;
        let ret = unsafe { xl_dl_get_finished_tasks(null_mut(), &mut count) };
        if ret != 0 {
            return Err(format!("获取失败： {}", get_error_msg(ret)));
        }
        let mut task_id_array = vec![0u64; count as usize];
        let ret = unsafe { xl_dl_get_finished_tasks(task_id_array.as_mut_ptr(), &mut count) };
        if ret != 0 {
            return Err(format!("获取失败： {}", get_error_msg(ret)));
        }

        Ok(task_id_array)
    }
    ///创建 p2sp 下载任务。
    pub fn create_task(
        &self,
        url: &str,
        save_path: &str,
        save_name: Option<&str>,
    ) -> Result<u64, String> {
        let name = if let Some(name) = save_name {
            name.to_string()
        } else {
            let path = std::path::Path::new(url);
            path.file_name()
                .map(|f| f.to_string_lossy().to_string())
                .ok_or("无法截取文件名".to_string())?
        };

        let url = to_cstring(url)?;
        let save_path = to_cstring(save_path)?;
        let save_name = to_cstring(&name)?;
        let mut task_id: u64 = 0;
        let ret = unsafe {
            xl_dl_create_p2sp_task(
                &mut xl_dl_create_p2sp_info {
                    url: url.as_ptr(),
                    save_name: save_name.as_ptr(),
                    save_path: save_path.as_ptr(),
                },
                &mut task_id,
            )
        };
        if ret != 0 {
            return Err(format!("创建任务失败: {}", get_error_msg(ret)));
        }
        Ok(task_id)
    }
    ///创建批量 p2sp 下载任务。
    pub fn create_batch_task(&self, batch_info: BatchInfo) -> Result<u64, String> {
        // --- 1. 处理最内层的 FileItem 数组 ---
        // 我们需要把所有的 String 转成 CString，并存起来防止被销毁
        let mut c_urls = Vec::new();
        let mut c_paths = Vec::new();
        let mut c_names = Vec::new();
        let mut c_hashes = Vec::new();

        for item in &batch_info.batch_files.file_items {
            c_urls.push(to_cstring(&item.url)?);
            c_paths.push(to_cstring(&item.save_path)?);
            c_names.push(to_cstring(&item.save_name)?);
            // 处理 Option Hash
            c_hashes.push(match &item.file_hash {
                Some(h) => Some(to_cstring(h)?),
                None => None,
            });
        }

        // --- 2. 构建 xl_dl_file_item_t 数组 ---
        let mut c_file_items: Vec<xl_dl_file_item_t> = c_urls
            .iter()
            .enumerate()
            .map(|(i, _)| xl_dl_file_item_t {
                url: c_urls[i].as_ptr(),
                save_path: c_paths[i].as_ptr(),
                save_name: c_names[i].as_ptr(),
                file_hash: c_hashes[i]
                    .as_ref()
                    .map_or(std::ptr::null(), |h| h.as_ptr()),
            })
            .collect();

        let mut c_files_info = xl_dl_files_info_t {
            file_count: c_file_items.len() as u32,
            file_list: c_file_items.as_mut_ptr(), // 指向数组首地址
        };

        let c_task_name = to_cstring(&batch_info.task_name)?;
        let c_create_info = xl_dl_create_batch_info_t {
            task_name: c_task_name.as_ptr(),
            max_concurrent: batch_info.max_concurrent,
            batch_files: &mut c_files_info,
        };

        let mut task_id: u64 = 0;
        unsafe {
            let ret = xl_dl_create_batch_task(&c_create_info, &mut task_id);
            if ret != 0 {
                return Err(format!("创建批量任务失败: {}", get_error_msg(ret)));
            }
        }

        Ok(task_id)
    }
    /// 启动任务。
    /// tid 任务id
    pub fn start_task(&self, tid: u64) -> Result<(), String> {
        let ret = unsafe { xl_dl_start_task(tid) };
        if ret != 0 {
            return Err(format!("任务开始失败： {}", get_error_msg(ret)));
        }
        Ok(())
    }
    /// 停止任务。
    /// tid 任务id
    pub fn stop_task(&self, tid: u64) -> Result<(), String> {
        let ret = unsafe { xl_dl_stop_task(tid) };
        if ret != 0 {
            return Err(format!("任务停止失败： {}", get_error_msg(ret)));
        }
        Ok(())
    }
    /// 删除任务。
    /// tid 任务id
    /// delete_file 是否删除文件
    pub fn delete_task(&self, tid: u64, delete_file: bool) -> Result<(), String> {
        let f = if delete_file { 1 } else { 0 };
        let ret = unsafe { xl_dl_delete_task(tid, f) };
        if ret != 0 {
            return Err(format!("任务删除失败： {}", get_error_msg(ret)));
        }
        Ok(())
    }

    /// 获取任务状态。
    /// tid 任务id
    pub fn get_task_state(&self, tid: u64) -> Result<TaskState, String> {
        // 1. 先准备一个 C 结构体（篮子），初始值设为全 0
        let mut c_state = unsafe { std::mem::zeroed::<xl_dl_task_state_t>() };

        // 2. 将这个 C 结构体的引用传给 C 函数
        let ret = unsafe { xl_dl_get_task_state(tid, &mut c_state) };
        if ret != 0 {
            return Err(format!("获取状态失败：{}", get_error_msg(ret)));
        }
        Ok(TaskState::from(c_state))
    }

    /// 获取任务信息。
    /// tid 任务id
    pub fn get_task_info(&self, tid: u64) -> Result<TaskInfo, String> {
        let url = get_task_property::<String>(tid, "url")?;
        let save_path = get_task_property::<String>(tid, "save_path")?;
        let save_name = get_task_property::<String>(tid, "save_name")?;
        let traffic = get_task_property::<TrafficInfo>(tid, "traffic")?;
        let creation_time = get_task_property::<u64>(tid, "creation_time")?;
        let completion_time = get_task_property::<u64>(tid, "completion_time")?;
        Ok(TaskInfo {
            url,
            save_path,
            save_name,
            traffic,
            creation_time,
            completion_time,
        })
    }

    /// 设置最大同时进行的下载任务数，默认 10
    pub fn set_concurrent_task_count(&self, count: u32) -> Result<(), String> {
        let ret = unsafe { xl_dl_set_concurrent_task_count(count) };
        if ret != 0 {
            return Err(format!(
                "设置最大同时进行的下载任务数失败：{}",
                get_error_msg(ret)
            ));
        }
        Ok(())
    }

    ///设置下载限速，默认不限速。
    pub fn set_download_speed_limit(&self, speed: u32) -> Result<(), String> {
        let ret = unsafe { xl_dl_set_download_speed_limit(speed) };
        if ret != 0 {
            return Err(format!("设置下载限速失败：{}", get_error_msg(ret)));
        }
        Ok(())
    }

    /// 设置是否开启 P2P 上传，默认不开启，设置优先级：API调用 > 控制台设置。
    pub fn set_upload_switch(&self, upload_switch: bool) -> Result<(), String> {
        let s = if upload_switch { 1 } else { 0 };
        let ret = unsafe { xl_dl_set_upload_switch(s) };
        if ret != 0 {
            return Err(format!("切换P2P上传失败：{}", get_error_msg(ret)));
        }
        Ok(())
    }

    /// 设置上传限速，默认不限速，设置优先级：API 调用 > 控制台设置。
    pub fn set_upload_speed_limit(&self, speed: u32) -> Result<(), String> {
        let ret = unsafe { xl_dl_set_upload_speed_limit(speed) };
        if ret != 0 {
            return Err(format!("设置上传限速失败：{}", get_error_msg(ret)));
        }
        Ok(())
    }

    pub fn set_http_headers(
        &self,
        tid: u64,
        headers: HashMap<String, String>,
    ) -> Result<(), String> {
        for (k, v) in headers.iter() {
            self.set_http_header(tid, k, v)?;
        }
        Ok(())
    }

    /// 该接口用于为下载请求设置 HTTP 头部信息。
    /// 当下载服务器要求请求中包含特定的 HTTP 头部字段（如 Referer）时，开发者可调用此接口为指定的下载任务设置相应的 HTTP 头部键值对。
    pub fn set_http_header(&self, tid: u64, k: &str, v: &str) -> Result<(), String> {
        unsafe {
            let key = to_cstring(k)?;
            let val = to_cstring(v)?;
            let ret = xl_dl_set_http_header(tid, key.as_ptr(), val.as_ptr());
            if ret != 0 {
                return Err(format!("设置 Header {} 失败: {}", k, get_error_msg(ret)));
            }
        }
        Ok(())
    }
    /// 查询 sdk 版本号。
    pub fn version() -> Result<String, i32> {
        let mut version_buffer = [0u8; 128];
        let mut version_len: u32 = version_buffer.len() as u32;

        unsafe {
            let code = xl_dl_version(version_buffer.as_mut_ptr() as *mut c_char, &mut version_len);
            if code == 0 {
                let c_str = CStr::from_ptr(version_buffer.as_ptr() as *const c_char);
                Ok(c_str.to_string_lossy().into_owned())
            } else {
                Err(code)
            }
        }
    }
}

pub fn to_cstring(s: &str) -> Result<CString, String> {
    let s = CString::new(s).map_err(|err| err.to_string())?;
    Ok(s)
}

#[derive(Debug)]
pub struct TaskState {
    pub speed: u64,
    pub total_size: u64,
    pub downloaded_size: u64,
    pub state_code: String,
    pub task_err_code: u32,
    pub task_token_err: u32,
}

impl From<xl_dl_task_state_t> for TaskState {
    fn from(value: xl_dl_task_state_t) -> Self {
        Self {
            speed: value.speed,
            total_size: value.total_size,
            downloaded_size: value.downloaded_size,
            state_code: get_task_state_msg(value.state_code).to_string(),
            task_err_code: value.task_err_code,
            task_token_err: value.task_token_err,
        }
    }
}
