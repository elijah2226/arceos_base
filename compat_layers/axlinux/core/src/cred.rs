
#[derive(Debug, Clone)]
pub struct Credentials {
    pub uid: u32,
    pub euid: u32,
    pub gid: u32,
    pub egid: u32,
}

impl Default for Credentials {
    fn default() -> Self {
        Self { uid: 0, euid: 0, gid: 0, egid: 0 }
    }
}