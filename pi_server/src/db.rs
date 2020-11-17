use crate::{api::RResult, client::CliJob};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTime;

struct CliMeta {
    jobs: Vec<CliJob>,
    login: Option<String>,
    last_cmd_ts: SystemTime,
}

type CDB = HashMap<SocketAddr, CliMeta>;

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

    fn check_cmd_timeout(addr: &SocketAddr) -> RResult<()> {
        let last_cmd_ts: SystemTime = Self::_lock_read().get(addr).unwrap().last_cmd_ts;
        if last_cmd_ts.elapsed().unwrap().as_secs() <= 1 {
            return Err("Too fast".to_string());
        } else {
            Self::_lock_write().get_mut(addr).unwrap().last_cmd_ts = SystemTime::now();
            return Ok(());
        }
    }

    pub fn init(addr: SocketAddr) {
        let cli_meta = CliMeta {
            jobs: vec![],
            login: None,
            last_cmd_ts: SystemTime::now(),
        };
        Self::_lock_write().insert(addr, cli_meta);
    }

    pub fn get_all_client_jobs(addr: &SocketAddr) -> Vec<CliJob> {
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

    pub fn add_job(addr: &SocketAddr, job: CliJob) -> RResult<()> {
        Self::_lock_write().get_mut(addr).unwrap().jobs.push(job);
        Ok(())
    }

    pub fn add_broadcast_job(addr_from: &SocketAddr, job: CliJob) -> RResult<()> {
        Self::check_cmd_timeout(addr_from)?;
        let addrs = Self::_lock_read()
            .keys()
            .map(|k| *k)
            .collect::<Vec<SocketAddr>>();
        addrs
            .into_iter()
            .for_each(|addr| ClientDB::add_job(&addr, job.clone()).unwrap());
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
}
