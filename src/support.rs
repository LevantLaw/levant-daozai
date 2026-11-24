//! 跨功能的进程、时钟、资源与测试辅助。
pub mod child_process;
pub mod poll_clock;
pub mod resources;
#[cfg(test)]
pub(crate) mod test_support;
