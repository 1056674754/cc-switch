//! 主线程看门狗
//!
//! 症状是「点一下，转圈，半天才进去」。沙滩球只会在 **AppKit 主线程**被同步阻塞时出现，
//! 而这类停顿往往只在真实负载下才复现，事后 `sample` 什么也抓不到。
//!
//! 这里用一条独立的线程，每 [`PROBE_INTERVAL`] 往主线程队列投一个空闭包：
//! 闭包在 [`STALL_THRESHOLD`] 内跑完就说明主线程正常；跑不完就说明它卡住了，
//! 立刻用 `/usr/bin/sample` 采样**自己这个进程**（同用户采样不需要任何权限），
//! 把当时的调用栈存到 `~/.cc-switch/logs/hang/`。
//!
//! 看门狗自己只碰通道和文件，不碰数据库、不碰任何锁，不会反过来被堵住。

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

/// 两次探测的间隔。往主线程队列投一个空闭包，开销可以忽略。
const PROBE_INTERVAL: Duration = Duration::from_millis(250);
/// 超过这个时长主线程还没跑探针，判定为卡顿。
const STALL_THRESHOLD: Duration = Duration::from_millis(1000);
/// 卡住之后等它恢复的最长时间（只影响日志里记录的时长）。
const RECOVER_WAIT: Duration = Duration::from_secs(120);
/// 启动宽限：初始化本身就要几秒，别把它当成卡顿。
const STARTUP_GRACE: Duration = Duration::from_secs(15);
/// 两次抓栈之间的最小间隔，避免连续卡顿时堆一堆采样进程。
const DUMP_COOLDOWN: Duration = Duration::from_secs(10);
/// 保留最近的抓栈份数。
const MAX_DUMPS: usize = 10;

static CAPTURING: AtomicBool = AtomicBool::new(false);

/// 启动看门狗。在 `.setup()` 里调用一次。
pub fn spawn(app: tauri::AppHandle) {
    let watchdog_app = app.clone();
    let result = std::thread::Builder::new()
        .name("main-thread-watchdog".into())
        .spawn(move || run(watchdog_app));
    if let Err(e) = result {
        log::warn!("[watchdog] 启动失败: {e}");
        return;
    }

    // 自检：`CCS_WATCHDOG_SELFTEST=1` 启动时故意把主线程按住 3 秒，
    // 用来确认「抓栈 -> ~/.cc-switch/logs/hang/」这条链路是通的。
    if std::env::var_os("CCS_WATCHDOG_SELFTEST").is_some() {
        let handle = app;
        std::thread::spawn(move || {
            std::thread::sleep(STARTUP_GRACE + Duration::from_secs(2));
            log::info!("[watchdog] 自检：把主线程按住 3 秒");
            let _ = handle.run_on_main_thread(|| std::thread::sleep(Duration::from_secs(3)));
        });
    }
}

fn run(app: tauri::AppHandle) {
    std::thread::sleep(STARTUP_GRACE);
    let mut last_dump: Option<Instant> = None;

    loop {
        let (tx, rx) = mpsc::channel::<()>();
        let started = Instant::now();
        if app
            .run_on_main_thread(move || {
                let _ = tx.send(());
            })
            .is_err()
        {
            // 事件循环已经退出（应用正在关闭）
            return;
        }

        match rx.recv_timeout(STALL_THRESHOLD) {
            Ok(()) => std::thread::sleep(PROBE_INTERVAL),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let cooling = last_dump.is_some_and(|t| t.elapsed() < DUMP_COOLDOWN);
                if !cooling {
                    last_dump = Some(Instant::now());
                    capture();
                }
                match rx.recv_timeout(RECOVER_WAIT) {
                    Ok(()) => log::error!(
                        "[watchdog] 主线程停顿 {} ms（调用栈见 {}）",
                        started.elapsed().as_millis(),
                        hang_dir().display()
                    ),
                    Err(_) => log::error!(
                        "[watchdog] 主线程停顿超过 {}s 仍未恢复",
                        RECOVER_WAIT.as_secs()
                    ),
                }
                std::thread::sleep(PROBE_INTERVAL);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn hang_dir() -> std::path::PathBuf {
    crate::config::get_app_config_dir().join("logs").join("hang")
}

/// 给自己这个进程抓一份调用栈。`sample` 会短暂挂起进程，因此放到子进程里跑，
/// 不阻塞看门狗线程。
fn capture() {
    if CAPTURING.swap(true, Ordering::AcqRel) {
        return;
    }

    let dir = hang_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        CAPTURING.store(false, Ordering::Release);
        log::warn!("[watchdog] 创建抓栈目录失败: {e}");
        return;
    }

    let stamp = local_stamp();
    let file = dir.join(format!("hang-{stamp}.txt"));
    let child = std::process::Command::new("/usr/bin/sample")
        .arg(std::process::id().to_string())
        .arg("3")
        .arg("-file")
        .arg(&file)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match child {
        Ok(mut child) => {
            let dir = dir.clone();
            std::thread::spawn(move || {
                let _ = child.wait();
                CAPTURING.store(false, Ordering::Release);
                prune(&dir);
            });
            append_stall_log(&format!("{stamp} 主线程停顿，采样 -> {}", file.display()));
        }
        Err(e) => {
            CAPTURING.store(false, Ordering::Release);
            log::warn!("[watchdog] 启动 sample 失败: {e}");
        }
    }
}

fn local_stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn append_stall_log(line: &str) {
    let path = hang_dir().join("stalls.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{line}");
    }
}

/// 只留最近 [`MAX_DUMPS`] 份抓栈文件。
fn prune(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut dumps: Vec<(SystemTime, std::path::PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_name()?.to_string_lossy().to_string();
            if name.starts_with("hang-") && name.ends_with(".txt") {
                let mtime = e.metadata().ok()?.modified().ok()?;
                Some((mtime, path))
            } else {
                None
            }
        })
        .collect();
    if dumps.len() <= MAX_DUMPS {
        return;
    }
    dumps.sort_by_key(|(mtime, _)| *mtime);
    for (_, path) in dumps.iter().take(dumps.len() - MAX_DUMPS) {
        let _ = std::fs::remove_file(path);
    }
}
