
📦 核心 API 说明
------------
**方法说明**
`init`
初始化全局下载引擎
`login`
鉴权并开启服务 Session
`create_task`
创建 P2SP 下载任务 (支持 HTTP/FTP)
`create_batch_task`
批量创建任务，适合网盘离线下载场景
`get_task_state`
获取实时进度、下载速度、错误码
`set_download_speed_limit`
全局限制下载速度
`
use thunder::Thunder;
use std::thread;
use std::time::Duration;
从官方开放平台获取你的凭证
const APP_ID: &str = "xxxxx";
const VERSION: &str = "1.0.1";
const CON_PATH: &str = "D:/ThunderConfig"; // 引擎配置文件存放路径
const SAVE_TASKS: u8 = 1;                  // 退出时是否保存任务
const SECRET: &str = "xxxxxxxx";
const ISSUER: &str = "strIssuer";

fn main() -> Result<(), String> {
    // 1. 初始化引擎
    println!("正在初始化迅雷引擎...");
    let mut xunlei = Thunder::init(APP_ID, VERSION, CON_PATH, SAVE_TASKS)?;

    // 2. 登录并获取 Session (有效期设为 24 小时)
    xunlei.login(APP_ID, SECRET, ISSUER, 60 * 60 * 24)?;
    println!("登录成功！");

    // 3. 创建下载任务 
    // 注意：目前仅支持 HTTP/HTTPS/FTP。磁力链接暂不支持。
    let url = "https://example.com/file.zip"; 
    let save_dir = "D:/Downloads";
    
    let tid = xunlei.create_task(url, save_dir, Some("my_file.zip"))?;
    
    // 4. 开始任务
    xunlei.start_task(tid)?;
    println!("任务已启动, ID: {}", tid);

    // 5. 循环监控下载进度
    loop {
        // 获取任务详情（URL、流量信息等）
        let _info = xunlei.get_task_info(tid)?;
        // 获取实时状态（百分比、速度、状态码）
        let state = xunlei.get_task_state(tid)?;

        println!(
            "进度: {:.2}% | 速度: {} KB/s | 状态: {:?}", 
            state.progress, 
            state.speed / 1024, 
            state.status
        );

        if state.progress >= 100.0 {
            println!("下载完成！");
            break;
        }

        thread::sleep(Duration::from_secs(3));
    }

    Ok(())
}
`