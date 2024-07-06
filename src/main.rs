pub mod cli;
pub mod store;

use anyhow::anyhow;
use clap::Parser;
use cli::{CLIArgs, SubCommand};
use prettytable::{row, Table};
use store::{DelegateCommand, Repository};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

fn main() {
    exec().expect("");
}
fn exec() -> Result<(), anyhow::Error> {
    let cli_args = CLIArgs::parse();
    let repo = Repository::create()?;

    //check for subcommands
    match &cli_args.subcommand {
        Some(SubCommand::List) => {
            let out = repo.list()?;
            let mut table = Table::new();
            table.add_row(row![
                "PID",
                "COMMAND",
                "STD_OUT_PATH",
                "STD_IN_PATH",
                "STD_ERR_PATH",
                "GROUP"
            ]);
            for o in out {
                table.add_row(o.to_table_row());
            }
            table.printstd();
            return Ok(());
        }
        Some(SubCommand::Reset) => {
            let out = repo.list()?;
            let s = System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new()),
            );
            for o in out {
                // try killing everything first and then deleting db
                let res = o.kill(&s);
                if res.is_err() {
                    println!("Couldn't kill process {o:?}");
                }
            }
            repo.delete()?;
            println!("Erased all traces of delegate");
            return Ok(());
        }
        Some(SubCommand::Restart { group }) => {
            let out = if let Some(g_num) = group {
                repo.list_with_group(*g_num)?
            } else {
                repo.list()?
            };
            let s = System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new()),
            );
            for o in &out {
                // try killing everything first
                let res = o.kill(&s);
                if res.is_err() {
                    println!("Couldn't kill process {o:?}");
                }
                repo.set_delete(o)?;
            }
            for o in out {
                let new_cmd = o.clone_spawn()?;
                repo.insert(&new_cmd)?;
            }
            return Ok(());
        }
        None => {}
    }

    //then check for kill
    if let Some(s) = cli_args.kill {
        // supplied arg for kill
        // check if string is num, if it is eliminate by pid otherwise eliminate by first word of command
        let num_test = s.parse::<usize>();
        if let Ok(num) = num_test {
            let cmd = repo.get_by_pid(num);
            let s = System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new()),
            );
            if let Ok(d_cmd) = cmd {
                d_cmd.kill(&s)?;
                println!("Killed process {d_cmd:?}");
                repo.set_delete(&d_cmd)?;
            } else {
                let d_cmds = repo.list_with_group(num)?;
                for d_cmd in d_cmds {
                    d_cmd.kill(&s)?;
                    println!("Killed process {d_cmd:?}");
                    repo.set_delete(&d_cmd)?;
                }
            }
        } else {
            let out = repo.list_with_name(s.as_str())?;
            let s = System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new()),
            );
            for o in out {
                // try killing everything first and then deleting db
                let res = o.kill(&s);
                if res.is_err() {
                    println!("Couldn't kill process {o:?}");
                }
                repo.set_delete(&o)?;
            }
        }
        return Ok(());
    }

    if cli_args.delegate.is_empty() {
        return Err(anyhow!("no command to delegate"));
    }

    let cmd = DelegateCommand::spawn(cli_args.delegate.join(" "), cli_args.group)?;
    repo.insert(&cmd)?;
    println!("Successfully started");

    Ok(())
}
