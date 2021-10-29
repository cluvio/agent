#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
pub const ARCHIVE_TEMPLATE: &str = "agent-<VERSION>-aarch64-unknown-linux-musl.tar.xz";

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub const ARCHIVE_TEMPLATE: &str = "agent-<VERSION>-x86_64-unknown-linux-musl.tar.xz";

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub const ARCHIVE_TEMPLATE: &str = "agent-<VERSION>-aarch64-apple-darwin.dmg";

#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
pub const ARCHIVE_TEMPLATE: &str = "agent-<VERSION>-x86_64-apple-darwin.dmg";

#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
pub const ARCHIVE_TEMPLATE: &str = "agent-<VERSION>-x86_64-pc-windows-msvc.tar.xz";

#[cfg(unix)]
pub const AGENT: &str = "cluvio-agent";

#[cfg(windows)]
pub const AGENT: &str = "cluvio-agent.exe";

