use base64::{engine::general_purpose::STANDARD as Base64, Engine};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn cmd<I, S, P>(program: &str, args: I, dir: Option<P>) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    P: AsRef<Path>,
{
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    println!(" $> {:?}", cmd);
    let output = cmd.output().expect("failed to execute command");
    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        panic!("🚨 {}", String::from_utf8_lossy(&output.stderr));
    }
}

fn cog_build(model: &str) {
    cmd("cog", ["build", "-t", &format!("cog-{model}")], Some(format!(".maintain/cog/{model}")));
}

fn docker_run(model: &str, host_port: u16) -> String {
    cmd(
        "docker",
        ["run", "-d", "-p", &format!("{host_port}:5000"), &format!("cog-{model}")],
        None::<&str>,
    )
    .trim()
    .to_owned()
}

pub fn docker_port(container_id: &str) -> u16 {
    cmd("docker", ["port", container_id, "5000"], None::<&str>)
        .split(':')
        .last()
        .expect("port mapping should exist")
        .trim()
        .parse()
        .expect("port should be a number")
}

pub fn build_and_run(model: &str, port: Option<u16>) -> u16 {
    println!("Setting up {model}");
    cog_build(model);
    let container_id = docker_run(model, port.unwrap_or_default());
    let allocated_port = docker_port(&container_id);
    println!("Running {model} on {allocated_port}");
    allocated_port
}

pub fn encode_file(path: &str) -> String {
    let bytes =
        fs::read(PathBuf::from(path)).unwrap_or_else(|_| panic!("Couldn't find file {path})"));
    let mime = tree_magic_mini::from_u8(&bytes);
    format!("data:{mime};base64,{}", Base64.encode(bytes))
}
