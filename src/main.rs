use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use clap::ValueEnum;
use std::env;
use std::io::Read;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

const HYPR_ENV: &str = "HYPRLAND_INSTANCE_SIGNATURE";
const ACTIVE_WINDOW_CMD: &str = "j/activewindow";
const ACTIVE_WORKSPACE_CMD: &str = "j/activeworkspace";
const WORKSPACES_CMD: &str = "j/workspaces";

/// Listen for hyprland events and emit the associated value (as json)
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Which mode should we run this in (what hyprland event are we watching)
    #[arg(value_enum)]
    mode: Mode,

    /// Hyprland control socket path
    #[arg(short = 'c', long)]
    hypr_ctl_path: Option<PathBuf>,

    /// Hyprland events socket path
    #[arg(short = 'e', long)]
    hypr_evt_path: Option<PathBuf>,
}

#[derive(Clone, ValueEnum)]
enum Mode {
    ActiveWindow,
    ActiveWorkspace,
    Workspaces,
}

fn ctl_cmd(ctlsocketpath: &PathBuf, cmd: &str) -> Result<()> {
    let mut ctl = UnixStream::connect(ctlsocketpath.clone())
        .with_context(|| format!("Failed to connect to {:?}", ctlsocketpath))?;
    let mut read_buf = [0; 1024];
    let mut json_str = String::new();

    ctl.write(cmd.as_bytes())?;

    loop {
        let n = ctl.read(&mut read_buf)?;
        json_str.push_str(std::str::from_utf8(&read_buf[..n])?);
        if n < read_buf.len() {
            break;
        }
    }
    println!("{}", json_str.replace("\n", ""));

    Ok(())
}

fn match_cmds(buffer: &str, cmds: &[&str]) -> Result<bool> {
    for line in buffer.lines() {
        if let Some((cmd, _data)) = line.split_once(">>") {
            if cmds.contains(&cmd) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let hypr_instance = env::var(HYPR_ENV);

    if hypr_instance.is_err() && (args.hypr_ctl_path.is_none() || args.hypr_evt_path.is_none()) {
        panic!("Socket path not specified and {} not set", HYPR_ENV);
    }

    let hypr_instance = hypr_instance.unwrap();

    let ctlsocketpath = args.hypr_ctl_path.clone().unwrap_or(
        ["/", "tmp", "hypr", &hypr_instance, ".socket.sock"]
            .iter()
            .collect(),
    );
    let eventsocketpath = args.hypr_evt_path.unwrap_or(
        ["/", "tmp", "hypr", &hypr_instance, ".socket2.sock"]
            .iter()
            .collect(),
    );

    match args.mode {
        Mode::ActiveWindow => ctl_cmd(&ctlsocketpath, ACTIVE_WINDOW_CMD)?,
        Mode::ActiveWorkspace => ctl_cmd(&ctlsocketpath, ACTIVE_WORKSPACE_CMD)?,
        Mode::Workspaces => ctl_cmd(&ctlsocketpath, WORKSPACES_CMD)?,
    }

    let mut events = UnixStream::connect(eventsocketpath.clone())
        .with_context(|| format!("Failed to connect to {:?}", eventsocketpath))?;
    let mut read_buf = [0; 1024];

    while let Ok(n) = events.read(&mut read_buf[..]) {
        match args.mode {
            Mode::ActiveWindow
                if match_cmds(
                    std::str::from_utf8(&read_buf[..n])?,
                    &["activewindow", "windowtitle"],
                )? =>
            {
                ctl_cmd(&ctlsocketpath, ACTIVE_WINDOW_CMD)?
            }
            Mode::ActiveWorkspace
                if match_cmds(
                    std::str::from_utf8(&read_buf[..n])?,
                    &["workspace", "activewindow"],
                )? =>
            {
                ctl_cmd(&ctlsocketpath, ACTIVE_WORKSPACE_CMD)?
            }
            Mode::Workspaces
                if match_cmds(
                    std::str::from_utf8(&read_buf[..n])?,
                    &["createworkspace", "destroyworkspace"],
                )? =>
            {
                ctl_cmd(&ctlsocketpath, WORKSPACES_CMD)?
            }
            _ => (),
        }
    }

    Ok(())
}
