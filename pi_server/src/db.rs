use crate::{api::RResult, client::CliTask, error::SError};
use std::net::SocketAddr;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTime;
use uuid::Uuid;

struct CliData {
    addr: SocketAddr,
    uid: Uuid,
    jobs: Vec<CliTask>,
    login: Option<String>,
    last_cmd_ts: SystemTime,
    password: Option<String>,
    online: bool,
}

type CDB = Vec<CliData>;

lazy_static! {
    static ref DB: RwLock<CDB> = RwLock::new(vec![]);
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
        Self::_lock_write()
            .iter_mut()
            .find(|cli| cli.addr == *addr)
            .unwrap()
            .last_cmd_ts = SystemTime::now();
    }

    fn check_cmd_timeout(addr: &SocketAddr, update: bool) -> RResult<()> {
        let last_cmd_ts: SystemTime = Self::_lock_read()
            .iter()
            .find(|cli| cli.addr == *addr)
            .unwrap()
            .last_cmd_ts;
        if last_cmd_ts.elapsed().unwrap().as_secs() < 1 {
            return Err(SError::DOS);
        } else {
            if update {
                Self::update_cmd_ts(addr);
            }
            return Ok(());
        }
    }

    pub fn add_client(addr: SocketAddr) {
        let cli_meta = CliData {
            addr: addr.clone(),
            uid: Uuid::new_v4(),
            jobs: vec![],
            login: None,
            last_cmd_ts: SystemTime::now(),
            password: None,
            online: true,
        };
        Self::_lock_write().push(cli_meta);
    }

    pub fn get_all_client_jobs(addr: &SocketAddr) -> Vec<CliTask> {
        Self::_lock_write()
            .iter_mut()
            .find(|cli| cli.addr == *addr)
            .unwrap()
            .jobs
            .drain(..)
            .collect()
    }

    pub fn get_all_users(addr: &SocketAddr) -> Vec<String> {
        Self::_lock_read()
            .iter()
            .map(|cli| {
                let mut user = cli.login.clone().unwrap_or(cli.addr.to_string());
                if *addr == cli.addr {
                    user += " (you)"
                }
                if cli.online {
                    user += " *"
                }
                user
            })
            .collect()
    }

    pub fn get_username(addr: &SocketAddr) -> Option<String> {
        Self::_lock_read()
            .iter()
            .find(|c| c.addr == *addr)
            .expect(&format!("can't find {}", addr))
            .login
            .clone()
    }

    pub fn get_client_by_username(username: String) -> Option<SocketAddr> {
        Self::_lock_read()
            .iter()
            .find(|cli| cli.login.is_some() && cli.login.as_ref().unwrap() == &username)
            .map(|cli| cli.addr)
    }

    pub fn add_task(addr: &SocketAddr, job: CliTask, has_timeout: bool) -> RResult<()> {
        Self::check_cmd_timeout(addr, has_timeout)?;
        Self::_lock_write()
            .iter_mut()
            .find(|cli| cli.addr == *addr)
            .unwrap()
            .jobs
            .push(job);
        Ok(())
    }

    pub fn add_broadcast_task(addr_from: &SocketAddr, job: CliTask) -> RResult<()> {
        Self::check_cmd_timeout(addr_from, false)?;
        let addrs = Self::_lock_read()
            .iter()
            .map(|cli| cli.addr)
            .collect::<Vec<SocketAddr>>();
        for addr in addrs.into_iter() {
            ClientDB::add_task(&addr, job.clone(), false)?;
        }
        Self::update_cmd_ts(addr_from);
        Ok(())
    }

    pub fn remove_cli(addr: &SocketAddr) {
        Self::_lock_write().retain(|cli| cli.addr != *addr);
    }

    pub fn remove_cli_by_uid(uid: &Uuid) {
        Self::_lock_write().retain(|cli| cli.uid != *uid);
    }

    pub fn set_online_status(addr: &SocketAddr, online: bool) {
        Self::_lock_write()
            .iter_mut()
            .find(|cli| cli.addr == *addr)
            .unwrap()
            .online = online;
    }

    pub fn set_login(addr: &SocketAddr, login: String, password: String) -> RResult<()> {
        if Self::is_logged_in(addr) {
            if Self::_lock_read()
                .iter()
                .any(|cli| cli.login.as_ref() == Some(&login) && cli.addr != *addr)
            {
                return Err(SError::LoginAlreadyExists);
            }
            if let Some(client) = Self::_lock_write().iter_mut().find(|cli| cli.addr == *addr) {
                client.login = Some(login);
                client.password = Some(password);
                client.online = true;
            }
            Ok(())
        } else {
            let mut del_old: Option<Uuid> = None;
            let del_uid = Self::_lock_read()
                .iter()
                .find(|cli| cli.addr == *addr)
                .unwrap()
                .uid;
            if let Some(cli) = Self::_lock_write()
                .iter_mut()
                .find(|cli| cli.login.as_ref() == Some(&login))
            {
                if cli.password.as_ref() != Some(&password) {
                    return Err(SError::WrongPassword);
                }
                if cli.online {
                    return Err(SError::AlreadyLoggedIn);
                }
                del_old = Some(del_uid);
                cli.addr = *addr;
                cli.online = true;
            }
            if del_old.is_some() {
                Self::remove_cli_by_uid(&del_old.unwrap());
                return Ok(());
            }
            if let Some(client) = Self::_lock_write().iter_mut().find(|cli| cli.addr == *addr) {
                client.login = Some(login);
                client.password = Some(password);
                client.online = true;
            }
            Ok(())
        }
    }

    pub fn is_logged_in(addr: &SocketAddr) -> bool {
        Self::_lock_read()
            .iter()
            .find(|c| c.addr == *addr)
            .unwrap()
            .login
            .is_some()
    }
}
