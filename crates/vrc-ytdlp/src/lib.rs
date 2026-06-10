//! Media proxy bridging VRChat with yt-dlp.
//!
//! The crate has two halves:
//!
//! - **CLI wrapper** — invoked by VRChat as a yt-dlp stand-in. Sanitizes
//!   untrusted arguments ([`args`]), keeps yt-dlp up to date
//!   ([`downloader`]), and either runs yt-dlp directly ([`executor`]) or
//!   routes `--get-url` requests through the media server
//!   ([`server::client`]). Entry point: [`cli::run`].
//! - **Media server** (`--serve`) — a local Axum HTTP server that resolves
//!   registered streams through a yt-dlp → ffmpeg pipeline and streams
//!   fragmented MP4 with a disk cache ([`server`]).

pub mod args;
pub mod cli;
pub mod config;
pub mod downloader;
pub mod executor;
pub mod logging;
pub mod paths;
pub mod server;
pub mod util;
