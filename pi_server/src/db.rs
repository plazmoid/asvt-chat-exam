use crate::{api::RResult, client::CliTask};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTime;

struct CliData {
    jobs: Vec<CliTask>,
    login: Option<String>,
    last_cmd_ts: SystemTime,
}

type CDB = HashMap<SocketAddr, CliData>;

lazy_static! {
    static ref DB: RwLock<CDB> = RwLock::new(HashMap::new());
}

pub struct ClientDB;

impl ClientDB {
    fn _lock_read() -> RwLockReadGuard<'static, CDB> {
        DB.read().unwrap()
    }

    fn _lock_write() -> RwLockWriteGuard<'static, CDB> {
        DB.write().unwrap()
    }

    fn update_cmd_ts(addr: &SocketAddr) {
        Self::_lock_write().get_mut(addr).unwrap().last_cmd_ts = SystemTime::now();
    }

    fn check_cmd_timeout(addr: &SocketAddr, update: bool) -> RResult<()> {
        let last_cmd_ts: SystemTime = Self::_lock_read().get(addr).unwrap().last_cmd_ts;
        if last_cmd_ts.elapsed().unwrap().as_secs() < 1 {
            return Err("Too fast".to_string());
        } else {
            if update {
                Self::update_cmd_ts(addr);
            }
            return Ok(());
        }
    }

    pub fn init(addr: SocketAddr) {
        let cli_meta = CliData {
            jobs: vec![],
            login: None,
            last_cmd_ts: SystemTime::now(),
        };
        Self::_lock_write().insert(addr, cli_meta);
    }

    pub fn get_all_client_jobs(addr: &SocketAddr) -> Vec<CliTask> {
        Self::_lock_write()
            .get_mut(addr)
            .unwrap()
            .jobs
            .drain(..)
            .collect()
    }

    pub fn get_all_users(addr: &SocketAddr) -> Vec<String> {
        Self::_lock_read()
            .iter()
            .map(|(_addr, cm)| {
                cm.login.clone().unwrap_or(_addr.to_string())
                    + if addr == _addr { " (you)" } else { "" }
            })
            .collect()
    }

    pub fn get_username(addr: &SocketAddr) -> Option<String> {
        Self::_lock_read().get(addr).unwrap().login.clone()
    }

    pub fn get_client_by_username(username: String) -> Option<SocketAddr> {
        Self::_lock_read()
            .iter()
            .find(|(_, v)| v.login.is_some() && v.login.as_ref().unwrap() == &username)
            .map(|(k, _)| *k)
    }

    pub fn add_task(addr: &SocketAddr, job: CliTask, has_timeout: bool) -> RResult<()> {
        Self::check_cmd_timeout(addr, has_timeout)?;
        Self::_lock_write().get_mut(addr).unwrap().jobs.push(job);
        Ok(())
    }

    pub fn add_broadcast_task(addr_from: &SocketAddr, job: CliTask) -> RResult<()> {
        Self::check_cmd_timeout(addr_from, false)?;
        let addrs = Self::_lock_read()
            .keys()
            .map(|k| *k)
            .collect::<Vec<SocketAddr>>();
        for addr in addrs.into_iter() {
            ClientDB::add_task(&addr, job.clone(), false)?;
        }
        Self::update_cmd_ts(addr_from);
        Ok(())
    }

    pub fn remove_cli(addr: &SocketAddr) {
        Self::_lock_write().remove(addr);
    }

    pub fn set_login(addr: &SocketAddr, login: String) -> RResult<()> {
        if Self::_lock_read()
            .values()
            .any(|cm| cm.login.as_ref() == Some(&login))
        {
            Err(format!("Login '{}' is already picked", login))
        } else {
            Self::_lock_write().get_mut(addr).unwrap().login = Some(login);
            Ok(())
        }
    }

    pub fn is_logged_in(addr: &SocketAddr) -> bool {
        Self::_lock_read().get(addr).unwrap().login.is_some()
    }
}
