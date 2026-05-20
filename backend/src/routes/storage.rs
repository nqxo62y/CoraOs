use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;

use crate::models::auth::AuthenticatedUser;
use crate::services::audit;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct FtpStatus {
    pub installed: bool,
    pub running: bool,
    pub config: Vec<FtpConfigEntry>,
    pub users: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FtpConfigEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct FtpConfigUpdate {
    pub anonymous_enable: bool,
    pub local_enable: bool,
    pub write_enable: bool,
    pub chroot_local_user: bool,
    pub listen_port: u16,
    pub pasv_min_port: u16,
    pub pasv_max_port: u16,
    pub max_clients: u16,
}

#[derive(Debug, Deserialize)]
pub struct FtpUserRequest {
    pub username: String,
    pub password: String,
    pub directory: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RaidArray {
    pub name: String,
    pub level: String,
    pub state: String,
    pub size: String,
    pub devices: Vec<String>,
    pub active_devices: String,
}

#[derive(Debug, Deserialize)]
pub struct RaidCreateRequest {
    pub name: String,
    pub level: String,
    pub devices: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MountPoint {
    pub device: String,
    pub mount_point: String,
    pub filesystem: String,
    pub options: String,
    pub size: String,
    pub used: String,
    pub available: String,
    pub use_percent: String,
}

#[derive(Debug, Serialize)]
pub struct BlockDevice {
    pub name: String,
    pub size: String,
    pub device_type: String,
    pub mountpoint: String,
    pub filesystem: String,
}

#[derive(Debug, Deserialize)]
pub struct MountRequest {
    pub device: String,
    pub mount_point: String,
    pub filesystem: String,
    pub options: Option<String>,
    pub persistent: bool,
}

#[derive(Debug, Deserialize)]
pub struct UnmountRequest {
    pub mount_point: String,
}

pub async fn ftp_status(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<FtpStatus>, (StatusCode, Json<Value>)> {
    let installed = Command::new("which").arg("vsftpd").output()
        .map(|o| o.status.success()).unwrap_or(false);

    let running = if installed {
        Command::new("systemctl").args(["is-active", "--quiet", "vsftpd"]).status()
            .map(|s| s.success()).unwrap_or(false)
    } else {
        false
    };

    let config = if installed {
        parse_vsftpd_config()
    } else {
        vec![]
    };

    let users = get_ftp_users();

    Ok(Json(FtpStatus { installed, running, config, users }))
}

pub async fn ftp_install(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))));
    }

    let output = Command::new("sudo")
        .args(["-n", "apt-get", "install", "-y", "vsftpd"])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": stderr.trim()}))));
    }

    let _ = Command::new("sudo").args(["-n", "systemctl", "enable", "vsftpd"]).output();
    let _ = Command::new("sudo").args(["-n", "systemctl", "start", "vsftpd"]).output();

    let _ = audit::log_action(&state.db, Some(&auth_user.user_id), "ftp_install", Some("storage"), None, None).await;

    Ok(Json(json!({"message": "vsftpd installed and started"})))
}

pub async fn ftp_update_config(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(payload): Json<FtpConfigUpdate>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))));
    }

    let config_content = format!(
        "listen=YES\nlisten_ipv6=NO\nanonymous_enable={}\nlocal_enable={}\n\
         write_enable={}\nchroot_local_user={}\nlisten_port={}\n\
         pasv_min_port={}\npasv_max_port={}\nmax_clients={}\n\
         pam_service_name=vsftpd\nssl_enable=NO\n",
        bool_to_yn(payload.anonymous_enable),
        bool_to_yn(payload.local_enable),
        bool_to_yn(payload.write_enable),
        bool_to_yn(payload.chroot_local_user),
        payload.listen_port,
        payload.pasv_min_port,
        payload.pasv_max_port,
        payload.max_clients,
    );

    let mut child = Command::new("sudo")
        .args(["-n", "tee", "/etc/vsftpd.conf"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(config_content.as_bytes());
    }
    let _ = child.wait();

    let _ = Command::new("sudo").args(["-n", "systemctl", "restart", "vsftpd"]).output();

    let _ = audit::log_action(&state.db, Some(&auth_user.user_id), "ftp_config_update", Some("storage"), None, None).await;

    Ok(Json(json!({"message": "FTP configuration updated and service restarted"})))
}

pub async fn ftp_add_user(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(payload): Json<FtpUserRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))));
    }

    if !payload.username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid username"}))));
    }

    let home_dir = payload.directory.unwrap_or_else(|| format!("/srv/ftp/{}", payload.username));

    let output = Command::new("sudo")
        .args(["-n", "useradd", "-m", "-d", &home_dir, "-s", "/usr/sbin/nologin", &payload.username])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": stderr.trim()}))));
    }

    let pass_input = format!("{}:{}", payload.username, payload.password);
    let mut child = Command::new("sudo")
        .args(["-n", "chpasswd"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(pass_input.as_bytes());
    }
    let _ = child.wait();

    let _ = audit::log_action(&state.db, Some(&auth_user.user_id), "ftp_user_create", Some(&payload.username), None, None).await;

    Ok(Json(json!({"message": format!("FTP user '{}' created", payload.username)})))
}

pub async fn ftp_delete_user(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))));
    }

    let _ = Command::new("sudo").args(["-n", "userdel", "-r", &name]).output();

    let _ = audit::log_action(&state.db, Some(&auth_user.user_id), "ftp_user_delete", Some(&name), None, None).await;

    Ok(Json(json!({"message": format!("FTP user '{}' deleted", name)})))
}

pub async fn raid_list(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<RaidArray>>, (StatusCode, Json<Value>)> {
    let arrays = parse_mdstat();
    Ok(Json(arrays))
}

pub async fn list_block_devices(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<BlockDevice>>, (StatusCode, Json<Value>)> {
    let output = Command::new("lsblk")
        .args(["-J", "-o", "NAME,SIZE,TYPE,MOUNTPOINT,FSTYPE"])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&stdout).unwrap_or(json!({"blockdevices": []}));

    let devices: Vec<BlockDevice> = parsed["blockdevices"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|d| BlockDevice {
            name: format!("/dev/{}", d["name"].as_str().unwrap_or("")),
            size: d["size"].as_str().unwrap_or("").to_string(),
            device_type: d["type"].as_str().unwrap_or("").to_string(),
            mountpoint: d["mountpoint"].as_str().unwrap_or("").to_string(),
            filesystem: d["fstype"].as_str().unwrap_or("").to_string(),
        })
        .collect();

    Ok(Json(devices))
}

pub async fn raid_create(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(payload): Json<RaidCreateRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))));
    }

    let valid_levels = ["0", "1", "5", "6", "10"];
    if !valid_levels.contains(&payload.level.as_str()) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid RAID level. Use: 0, 1, 5, 6, or 10"}))));
    }

    if payload.devices.len() < 2 {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "At least 2 devices required"}))));
    }

    for dev in &payload.devices {
        if !dev.starts_with("/dev/") || dev.contains("..") {
            return Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Invalid device: {}", dev)}))));
        }
    }

    let md_device = format!("/dev/md/{}", payload.name);
    let device_count = payload.devices.len().to_string();

    let mut args = vec![
        "-n".to_string(), "mdadm".to_string(),
        "--create".to_string(), md_device.clone(),
        "--level".to_string(), payload.level.clone(),
        "--raid-devices".to_string(), device_count,
    ];
    for dev in &payload.devices {
        args.push(dev.clone());
    }
    args.push("--run".to_string());

    let output = Command::new("sudo")
        .args(&args)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": stderr.trim()}))));
    }

    let _ = audit::log_action(&state.db, Some(&auth_user.user_id), "raid_create", Some(&payload.name), Some(&format!("Level: {}, Devices: {:?}", payload.level, payload.devices)), None).await;

    Ok(Json(json!({"message": format!("RAID array '{}' created (level {})", payload.name, payload.level)})))
}

pub async fn raid_delete(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))));
    }

    let md_device = format!("/dev/md/{}", name);
    let _ = Command::new("sudo").args(["-n", "mdadm", "--stop", &md_device]).output();
    let _ = Command::new("sudo").args(["-n", "mdadm", "--remove", &md_device]).output();

    let _ = audit::log_action(&state.db, Some(&auth_user.user_id), "raid_delete", Some(&name), None, None).await;

    Ok(Json(json!({"message": format!("RAID array '{}' stopped and removed", name)})))
}

pub async fn list_mounts(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<MountPoint>>, (StatusCode, Json<Value>)> {
    let output = Command::new("df")
        .args(["-hT", "--output=source,target,fstype,size,used,avail,pcent"])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mounts: Vec<MountPoint> = stdout
        .lines()
        .skip(1)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 7 {
                Some(MountPoint {
                    device: parts[0].to_string(),
                    mount_point: parts[1].to_string(),
                    filesystem: parts[2].to_string(),
                    options: String::new(),
                    size: parts[3].to_string(),
                    used: parts[4].to_string(),
                    available: parts[5].to_string(),
                    use_percent: parts[6].to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(Json(mounts))
}

pub async fn mount_device(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(payload): Json<MountRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))));
    }

    if !payload.device.starts_with("/dev/") || payload.device.contains("..") {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid device path"}))));
    }
    if !payload.mount_point.starts_with("/") || payload.mount_point.contains("..") {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid mount point"}))));
    }

    let _ = Command::new("sudo").args(["-n", "mkdir", "-p", &payload.mount_point]).output();

    let mut args = vec!["-n".to_string(), "mount".to_string()];
    if !payload.filesystem.is_empty() {
        args.push("-t".to_string());
        args.push(payload.filesystem.clone());
    }
    if let Some(ref opts) = payload.options {
        if !opts.is_empty() {
            args.push("-o".to_string());
            args.push(opts.clone());
        }
    }
    args.push(payload.device.clone());
    args.push(payload.mount_point.clone());

    let output = Command::new("sudo")
        .args(&args)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": stderr.trim()}))));
    }

    if payload.persistent {
        let fstab_line = format!(
            "{} {} {} {} 0 2\n",
            payload.device,
            payload.mount_point,
            payload.filesystem,
            payload.options.as_deref().unwrap_or("defaults")
        );
        let mut child = Command::new("sudo")
            .args(["-n", "tee", "-a", "/etc/fstab"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

        use std::io::Write;
        if let Some(ref mut stdin) = child.stdin {
            let _ = stdin.write_all(fstab_line.as_bytes());
        }
        let _ = child.wait();
    }

    let _ = audit::log_action(&state.db, Some(&auth_user.user_id), "mount", Some(&payload.mount_point), Some(&payload.device), None).await;

    Ok(Json(json!({"message": format!("Mounted {} at {}", payload.device, payload.mount_point)})))
}

pub async fn unmount_device(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(payload): Json<UnmountRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Admin access required"}))));
    }

    let output = Command::new("sudo")
        .args(["-n", "umount", &payload.mount_point])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": stderr.trim()}))));
    }

    let _ = audit::log_action(&state.db, Some(&auth_user.user_id), "unmount", Some(&payload.mount_point), None, None).await;

    Ok(Json(json!({"message": format!("Unmounted {}", payload.mount_point)})))
}

fn bool_to_yn(b: bool) -> &'static str {
    if b { "YES" } else { "NO" }
}

fn parse_vsftpd_config() -> Vec<FtpConfigEntry> {
    match std::fs::read_to_string("/etc/vsftpd.conf") {
        Ok(content) => content
            .lines()
            .filter(|l| !l.starts_with('#') && l.contains('='))
            .filter_map(|l| {
                let parts: Vec<&str> = l.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some(FtpConfigEntry {
                        key: parts[0].trim().to_string(),
                        value: parts[1].trim().to_string(),
                    })
                } else {
                    None
                }
            })
            .collect(),
        Err(_) => vec![],
    }
}

fn get_ftp_users() -> Vec<String> {
    match std::fs::read_to_string("/etc/passwd") {
        Ok(content) => content
            .lines()
            .filter(|l| l.contains("/srv/ftp/") || l.contains("vsftpd"))
            .filter_map(|l| l.split(':').next().map(|s| s.to_string()))
            .collect(),
        Err(_) => vec![],
    }
}

fn parse_mdstat() -> Vec<RaidArray> {
    let content = match std::fs::read_to_string("/proc/mdstat") {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut arrays = vec![];
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.starts_with("md") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let state = parts[2].to_string();
                let level = parts[3].to_string();
                let devices: Vec<String> = parts[4..].iter()
                    .map(|d| d.split('[').next().unwrap_or("").to_string())
                    .filter(|d| !d.is_empty())
                    .collect();

                let size = if let Some(next_line) = lines.peek() {
                    next_line.split_whitespace()
                        .find(|s| s.contains("blocks"))
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                arrays.push(RaidArray {
                    name,
                    level,
                    state,
                    size,
                    active_devices: format!("{}", devices.len()),
                    devices,
                });
            }
        }
    }

    arrays
}
