use crate::{api::RResult, client::CliTask, config::*, error::SError, utils::threaded_task_runner};
use std::fs::{File, OpenOptions};
use std::net::SocketAddr;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct CliData {
    addr: SocketAddr,
    uid: Uuid,
    jobs: Vec<CliTask>,
    login: Option<String>,
    last_cmd_ts: SystemTime,
    password: Option<String>,
    online: bool,
}

impl Default for CliData {
    fn default() -> CliData {
        CliData {
            addr: "127.0.0.1:31337".parse().unwrap(),
            uid: Uuid::new_v4(),
            jobs: vec![],
            login: None,
            last_cmd_ts: SystemTime::now(),
            password: None,
            online: false,
        }
    }
}

type CDB = Vec<CliData>;

lazy_static! {
    static ref DB: RwLock<CDB> = RwLock::new({
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(DB_PATH)
            .unwrap();
        let mut db: Vec<CliData> = serde_json::from_reader(file).unwrap_or(vec![]);
        db.iter_mut().for_each(|cli| cli.online = false);
        db
    });
    pub static ref _T: () =
        threaded_task_runner(|| ClientDB::sync_db(), Duration::from_millis(1200));
}

pub struct ClientDB;

impl ClientDB {
    pub fn _lock_read() -> RwLockReadGuard<'static, CDB> {
        DB.read().unwrap()
    }

    pub fn _lock_write() -> RwLockWriteGuard<'static, CDB> {
        DB.write().unwrap()
    }

    pub fn get_uid(addr: &SocketAddr) -> Option<Uuid> {
        Self::_lock_read()
            .iter()
            .find(|cli| cli.addr == *addr && cli.online)
            .map(|c| c.uid)
    }

    pub fn update_cmd_ts(uid: Uuid) {
        Self::_lock_write()
            .iter_mut()
            .find(|cli| cli.uid == uid)
            .unwrap()
            .last_cmd_ts = SystemTime::now();
    }

    pub fn check_cmd_timeout(uid: Uuid) -> RResult<()> {
        let last_cmd_ts: SystemTime = Self::_lock_read()
            .iter()
            .find(|cli| cli.uid == uid)
            .unwrap()
            .last_cmd_ts;
        if last_cmd_ts.elapsed().unwrap().as_millis() < 500 {
            return Err(SError::DOS);
        } else {
            Self::update_cmd_ts(uid);
            return Ok(());
        }
    }

    pub fn sync_db() {
        if let Err(e) = serde_json::to_writer(
            File::create(DB_PATH).unwrap(),
            &Self::_lock_read()
                .iter()
                .filter(|cli| cli.login.is_some())
                .collect::<Vec<&CliData>>(),
        ) {
            error!("Failed to dump db: {}", e);
        }
    }

    pub fn add_client(addr: SocketAddr) -> Uuid {
        let mut cli_meta = CliData::default();
        cli_meta.addr = addr.clone();
        cli_meta.online = true;
        let cli_uid = cli_meta.uid;
        Self::_lock_write().push(cli_meta);
        cli_uid
    }

    pub fn get_all_client_jobs(uid: Uuid) -> Option<Vec<CliTask>> {
        if Self::_lock_read()
            .iter()
            .find(|cli| cli.uid == uid)
            .unwrap_or(&CliData::default())
            .jobs
            .len()
            > 0
        {
            return Some(
                Self::_lock_write()
                    .iter_mut()
                    .find(|cli| cli.uid == uid)
                    .unwrap()
                    .jobs
                    .drain(..)
                    .collect(),
            );
        } else {
            None
        }
    }

    pub fn get_all_users(uid: Uuid) -> Vec<String> {
        Self::_lock_read()
            .iter()
            .map(|cli| {
                let mut user = cli.login.clone().unwrap_or(cli.addr.to_string());
                if cli.uid == uid {
                    user += " (you)"
                }
                if cli.online {
                    user = format!("{} {}", user, ONLINE);
                }
                user
            })
            .collect()
    }

    pub fn get_username(uid: Uuid) -> Option<String> {
        Self::_lock_read()
            .iter()
            .find(|c| c.uid == uid)
            .expect(&format!("can't find {}", uid))
            .login
            .clone()
    }

    pub fn get_client_by_username(username: &String) -> Option<Uuid> {
        Self::_lock_read()
            .iter()
            .find(|cli| cli.login.is_some() && cli.login.as_ref().unwrap() == username)
            .map(|cli| cli.uid)
    }

    pub fn add_task(uid: Uuid, task: CliTask) -> RResult<()> {
        Self::_lock_write()
            .iter_mut()
            .find(|cli| cli.uid == uid)
            .unwrap()
            .jobs
            .push(task);
        Ok(())
    }

    pub fn add_broadcast_task(uid: Uuid, task: CliTask) -> RResult<()> {
        let clients = Self::_lock_read()
            .iter()
            .map(|cli| cli.uid)
            .collect::<Vec<Uuid>>();
        for cli_uid in clients.into_iter() {
            ClientDB::add_task(cli_uid, task.clone())?;
        }
        Self::update_cmd_ts(uid);
        Ok(())
    }

    pub fn remove_cli(uid: Uuid) {
        Self::_lock_write().retain(|cli| cli.uid != uid);
    }

    pub fn set_online_status(uid: Uuid, online: bool) {
        Self::_lock_write()
            .iter_mut()
            .find(|cli| cli.uid == uid)
            .unwrap()
            .online = online;
    }

    pub fn set_login(uid: Uuid, addr: &SocketAddr, login: String, password: String) -> RResult<()> {
        if Self::is_logged_in(uid) {
            if Self::_lock_read()
                .iter()
                .any(|cli| cli.login.as_ref() == Some(&login) && cli.uid != uid)
            {
                return Err(SError::LoginAlreadyExists);
            }
            if let Some(client) = Self::_lock_write().iter_mut().find(|cli| cli.uid == uid) {
                client.login = Some(login);
                client.password = Some(password);
                client.online = true;
            }
            Ok(())
        } else {
            let mut perform_login = false;
            if let Some(cli) = Self::_lock_read()
                .iter()
                .find(|cli| cli.login.as_ref() == Some(&login))
            {
                if cli.password.as_ref() != Some(&password) {
                    return Err(SError::WrongPassword);
                }
                if cli.online {
                    return Err(SError::AlreadyLoggedIn);
                }
                perform_login = true;
            }
            if perform_login {
                Self::remove_cli(uid);
                if let Some(cli) = Self::_lock_write()
                    .iter_mut()
                    .find(|cli| cli.login.as_ref() == Some(&login))
                {
                    cli.addr = *addr;
                    cli.online = true;
                    cli.uid = uid;
                }
                return Ok(());
            }
            if let Some(client) = Self::_lock_write().iter_mut().find(|cli| cli.uid == uid) {
                client.login = Some(login);
                client.password = Some(password);
                client.online = true;
            }
            Ok(())
        }
    }

    pub fn is_logged_in(uid: Uuid) -> bool {
        Self::_lock_read()
            .iter()
            .find(|c| c.uid == uid)
            .unwrap_or(&CliData::default())
            .login
            .is_some()
    }
}
