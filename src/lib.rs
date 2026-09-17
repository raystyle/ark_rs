//! ark：Ark（Agent Runtime Kit）本机跨平台环境部署管理 CLI。
//! 三原语 doctor / install / status；派生 query、update、pin、verify、heal、
//! init、self update。catalog 为唯一 pin 源。
//! 下载镜像主通道 env.ohmygh.com、官方兜底（D44）；锁定归云端数据面（D37）。
//! 公开契约双面：命令输出走黄金文件 oracle，lib 公开项走 docs/aidoc/ 投影。

pub mod catalog;
pub mod checksum;
pub mod docker;
pub mod doctor;
pub mod download;
pub mod envpath;
pub mod extract;
pub mod heal;
pub mod install;
pub mod issue;
pub mod manifest;
pub mod omerr;
pub mod platform;
pub mod render;
pub mod resolve;
pub mod rustup;
pub mod selfdeploy;
pub mod selfupdate;
pub mod status;
pub mod toolver;
pub mod verify;
pub mod vsbuild;
