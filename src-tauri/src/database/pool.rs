//! SQLite 连接池
//!
//! 取代原先的 `Mutex<Connection>`：全进程只有一条连接时，代理写请求日志、用量聚合、
//! 备份和前端命令会排同一把锁，任何一次长查询都会让后面所有人等着。
//!
//! 这里只用 `std`，没有引入新的依赖。对外接口刻意做成和 `Mutex<Connection>` 同形——
//! 字段名仍叫 `conn`、取用仍叫 `lock()`、守卫可 `Deref` 成 `&Connection`，
//! 所以全仓 370 处 `lock_conn!(self.conn)` 无需改动。
//!
//! ## 并发事实
//!
//! 池子消除了 **Rust 层**的全局锁，读与读、读与写可以并行；但 SQLite 任何时刻只允许
//! 一个写事务，写与写仍然串行，靠 `busy_timeout` 排队而不是报错。WAL 模式是前提，
//! 见 `configure_connection`。

use rusqlite::Connection;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// 池上限。读多写少，8 条足够；再大只会多占内存、并让 WAL 的 checkpoint 更迟。
pub(crate) const DEFAULT_POOL_SIZE: usize = 8;

/// 取连接的最长等待时间。到点就报错返回，绝不无限阻塞。
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);

/// 单条连接的写锁等待时间（SQLite 层的 `busy_timeout`）。
pub(crate) const BUSY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub(crate) enum PoolError {
    /// 池子被占满且等待超时
    Timeout,
    /// 打开新连接失败
    Open(String),
    /// 内部互斥量被毒化（某条线程 panic 时发生过持锁）
    Poisoned,
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoolError::Timeout => write!(
                f,
                "连接池已满，等待 {}s 仍未取到连接",
                ACQUIRE_TIMEOUT.as_secs()
            ),
            PoolError::Open(e) => write!(f, "打开数据库连接失败: {e}"),
            PoolError::Poisoned => write!(f, "连接池内部锁已损坏"),
        }
    }
}

impl std::error::Error for PoolError {}

struct PoolInner {
    /// `None` = 单连接模式：内存库必须始终是同一条连接，不能新建。
    path: Option<PathBuf>,
    max: usize,
    /// 每条新建连接都要跑的配置（PRAGMA + 数据库变更钩子）。
    configure: fn(&Connection),
    idle: Mutex<Vec<Connection>>,
    /// 已创建、且没有被丢弃的连接条数。
    live: Mutex<usize>,
    cv: Condvar,
}

/// 借出的一条连接；`Drop` 时自动归还。
pub(crate) struct PooledConn<'a> {
    pool: &'a PoolInner,
    conn: Option<Connection>,
    /// 和 `MutexGuard` 一样不能 `Send` 走。否则「跨 `.await` 持着连接」这种本来
    /// 编译不过的写法会突然可用，把池子占住饿死别人。
    _not_send: std::marker::PhantomData<*const ()>,
}

impl std::ops::Deref for PooledConn<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.conn.as_ref().expect("连接已在 Drop 中取出")
    }
}

impl std::ops::DerefMut for PooledConn<'_> {
    fn deref_mut(&mut self) -> &mut Connection {
        self.conn.as_mut().expect("连接已在 Drop 中取出")
    }
}

impl Drop for PooledConn<'_> {
    fn drop(&mut self) {
        let Some(conn) = self.conn.take() else {
            return;
        };

        // 没跑完的事务必须丢弃：连接上的未提交状态会串给下一个借用人。
        if conn.is_autocommit() {
            if let Ok(mut idle) = self.pool.idle.lock() {
                idle.push(conn);
                drop(idle);
                self.pool.cv.notify_one();
                return;
            }
        }

        // 关闭这条连接（Drop 会回滚未提交事务），把名额还回去。
        drop(conn);
        if let Ok(mut live) = self.pool.live.lock() {
            *live = live.saturating_sub(1);
        }
        self.pool.cv.notify_one();
    }
}

/// 多连接池。`Database.conn` 的类型，用法与 `Mutex<Connection>` 一致。
pub(crate) struct ConnectionPool {
    inner: Arc<PoolInner>,
}

impl ConnectionPool {
    /// 文件库：按需建连接，最多 [`DEFAULT_POOL_SIZE`] 条。
    pub(crate) fn file(path: PathBuf, configure: fn(&Connection)) -> Result<Self, PoolError> {
        let inner = Arc::new(PoolInner {
            path: Some(path),
            max: DEFAULT_POOL_SIZE,
            configure,
            idle: Mutex::new(Vec::new()),
            live: Mutex::new(0),
            cv: Condvar::new(),
        });
        // 先建一条：初始化阶段的 PRAGMA（auto_vacuum 等）必须落在真实的连接上，
        // 而且建表之前池里只有这一条，顺序是确定的。
        let conn = inner.open()?;
        inner.idle.lock().map_err(|_| PoolError::Poisoned)?.push(conn);
        *inner.live.lock().map_err(|_| PoolError::Poisoned)? = 1;
        Ok(Self { inner })
    }

    /// 内存库：只有一条连接，永远不新建（`Connection::open_in_memory` 每次都会得到
    /// 一个各自独立的空库，多条连接会让约 100 个测试看到不同的数据）。
    pub(crate) fn memory(configure: fn(&Connection)) -> Result<Self, PoolError> {
        let conn = Connection::open_in_memory().map_err(|e| PoolError::Open(e.to_string()))?;
        configure(&conn);
        let inner = Arc::new(PoolInner {
            path: None,
            max: 1,
            configure,
            idle: Mutex::new(vec![conn]),
            live: Mutex::new(1),
            cv: Condvar::new(),
        });
        Ok(Self { inner })
    }

    /// 借一条连接；与 `Mutex::lock()` 同形，便于沿用 `lock_conn!` 宏。
    pub(crate) fn lock(&self) -> Result<PooledConn<'_>, PoolError> {
        self.inner.acquire()
    }

    /// 池上限（测试里用来一次占满整个池）。
    #[cfg(test)]
    pub(crate) fn max_size(&self) -> usize {
        self.inner.max
    }
}

impl PoolInner {
    fn open(&self) -> Result<Connection, PoolError> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| PoolError::Open("单连接池不能再打开新连接".into()))?;
        let conn = Connection::open(path).map_err(|e| PoolError::Open(e.to_string()))?;
        (self.configure)(&conn);
        Ok(conn)
    }

    fn acquire(&self) -> Result<PooledConn<'_>, PoolError> {
        let deadline = Instant::now() + ACQUIRE_TIMEOUT;

        loop {
            // 1. 有空闲连接就直接借出
            if let Some(conn) = self
                .idle
                .lock()
                .map_err(|_| PoolError::Poisoned)?
                .pop()
            {
                return Ok(PooledConn {
                    pool: self,
                    conn: Some(conn),
                    _not_send: std::marker::PhantomData,
                });
            }

            // 2. 没到上限就新开一条
            let mut live = self.live.lock().map_err(|_| PoolError::Poisoned)?;
            if *live < self.max {
                *live += 1;
                drop(live);
                return match self.open() {
                    Ok(conn) => Ok(PooledConn {
                        pool: self,
                        conn: Some(conn),
                        _not_send: std::marker::PhantomData,
                    }),
                    Err(e) => {
                        if let Ok(mut live) = self.live.lock() {
                            *live = live.saturating_sub(1);
                        }
                        self.cv.notify_one();
                        Err(e)
                    }
                };
            }
            drop(live);

            // 3. 等别人归还。先持 idle 锁再 wait，归还方也必须持 idle 锁才能入队，
            //    这样不会漏掉唤醒。
            let now = Instant::now();
            if now >= deadline {
                return Err(PoolError::Timeout);
            }
            let idle = self.idle.lock().map_err(|_| PoolError::Poisoned)?;
            let (_idle, _) = self
                .cv
                .wait_timeout(idle, deadline - now)
                .map_err(|_| PoolError::Poisoned)?;
        }
    }
}
