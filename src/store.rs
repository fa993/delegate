// sql has to have cmd, stdout file, stdin file, pid

use std::{
    fs::remove_file,
    process::{Command, Stdio},
};

use anyhow::anyhow;
use home::home_dir;
use prettytable::{row, Row};
use rusqlite::Connection;
use sysinfo::{Pid, System};
use tempfile::NamedTempFile;

const PATH: &'static str = ".delegatedb";

#[derive(Debug)]
pub struct DelegateCommand {
    pid: usize,
    command: String,
    stdout_path: String,
    stdin_path: String,
    stderr_path: String,
}

impl DelegateCommand {
    pub fn spawn(cmd: String) -> Result<DelegateCommand, anyhow::Error> {
        let out_file = NamedTempFile::new()?;
        let (o_file, o_path) = out_file.keep()?;

        let in_file = NamedTempFile::new()?;
        let (i_file, i_path) = in_file.keep()?;

        let err_file = NamedTempFile::new()?;
        let (e_file, e_path) = err_file.keep()?;

        let res = Command::new("bash")
            .arg("-c")
            .arg(cmd.clone())
            .stdin(Stdio::from(i_file))
            .stdout(Stdio::from(o_file))
            .stderr(Stdio::from(e_file))
            .spawn()?;

        Ok(DelegateCommand {
            pid: res.id() as usize,
            command: cmd,
            stdout_path: o_path.to_string_lossy().to_string(),
            stdin_path: i_path.to_string_lossy().to_string(),
            stderr_path: e_path.to_string_lossy().to_string(),
        })
    }
}

pub struct Repository {
    conn: Connection,
}

impl Repository {
    pub fn create() -> Result<Repository, anyhow::Error> {
        let home_dir = home_dir().ok_or(anyhow!("Couldn't get home dir"))?;
        let path = home_dir.join(PATH);
        let conn = Connection::open(path)?;

        conn.execute(
            "CREATE TABLE if not exists delegate_command (
            id INTEGER PRIMARY KEY, 
            pid INTEGER NOT NULL,
            command TEXT NOT NULL,
            stdout_path TEXT NOT NULL,
            stdin_path TEXT NOT NULL,
            stderr_path TEXT NOT NULL,
            ongoing INTEGER DEFAULT (1) NOT NULL
        )",
            (), // empty list of parameters.
        )?;

        return Ok(Repository { conn });
    }

    pub fn insert(&self, cmd: &DelegateCommand) -> Result<usize, anyhow::Error> {
        let result = self.conn.execute(
            "INSERT INTO delegate_command (pid, command, stdout_path, stdin_path, stderr_path) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                cmd.pid,
                cmd.command.as_str(),
                cmd.stdout_path.as_str(),
                cmd.stdin_path.as_str(),
                cmd.stderr_path.as_str()
            ),
        )?;
        Ok(result)
    }

    pub fn list(&self) -> Result<Vec<DelegateCommand>, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT pid, command, stdout_path, stdin_path, stderr_path FROM delegate_command where ongoing=1",
        )?;
        let cmd_iter = stmt.query_map([], |row| {
            Ok(DelegateCommand {
                pid: row.get(0)?,
                command: row.get(1)?,
                stdout_path: row.get(2)?,
                stdin_path: row.get(3)?,
                stderr_path: row.get(4)?,
            })
        })?;

        let mut out = Vec::new();

        for cmd in cmd_iter {
            out.push(cmd?);
        }

        return Ok(out);
    }

    pub fn list_with(&self, starts_with: &str) -> Result<Vec<DelegateCommand>, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT pid, command, stdout_path, stdin_path, stderr_path FROM delegate_command where ongoing=1 AND command LIKE ?1 || '%'",
        )?;
        let cmd_iter = stmt.query_map([starts_with], |row| {
            Ok(DelegateCommand {
                pid: row.get(0)?,
                command: row.get(1)?,
                stdout_path: row.get(2)?,
                stdin_path: row.get(3)?,
                stderr_path: row.get(4)?,
            })
        })?;

        let mut out = Vec::new();

        for cmd in cmd_iter {
            out.push(cmd?);
        }

        return Ok(out);
    }

    pub fn delete(self) -> Result<(), anyhow::Error> {
        let home_dir = home_dir().ok_or(anyhow!("Couldn't get home dir"))?;
        let path = home_dir.join(PATH);
        remove_file(path)?;
        Ok(())
    }

    pub fn get_by_pid(&self, pid: usize) -> Result<DelegateCommand, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT pid, command, stdout_path, stdin_path, stderr_path FROM delegate_command where ongoing=1 and pid=?1",
        )?;
        let cmd_iter = stmt.query_map([pid], |row| {
            Ok(DelegateCommand {
                pid: row.get(0)?,
                command: row.get(1)?,
                stdout_path: row.get(2)?,
                stdin_path: row.get(3)?,
                stderr_path: row.get(4)?,
            })
        })?;

        let mut out = Vec::new();

        for cmd in cmd_iter {
            out.push(cmd?);
        }

        if out.len() > 1 {
            return Err(anyhow!("More than 1 ongoing process for 1 PID"));
        }

        return Ok(out.pop().unwrap());
    }

    pub fn set_delete(&self, pr: &DelegateCommand) -> Result<(), anyhow::Error> {
        self.conn.execute(
            "UPDATE delegate_command SET ongoing=0 where pid=?1",
            [pr.pid],
        )?;
        Ok(())
    }
}

impl DelegateCommand {
    pub fn to_table_row(&self) -> Row {
        return row![
            self.pid.to_string(),
            self.command.to_string(),
            self.stdout_path.to_string(),
            self.stdin_path.to_string(),
            self.stderr_path.to_string(),
        ];
    }

    pub fn kill(&self, s: &System) -> Result<(), anyhow::Error> {
        let process = s
            .process(Pid::from(self.pid))
            .ok_or(anyhow!("process with PID not found"))?;
        process.kill();
        return Ok(());
    }
}
