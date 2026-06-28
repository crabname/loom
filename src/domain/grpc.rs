#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrpcMethodInfo {
    pub service: String,
    pub method: String,
}

impl GrpcMethodInfo {
    pub fn path(&self) -> String {
        format!("/{}/{}", self.service, self.method)
    }

    pub fn label(&self) -> String {
        format!("{}.{}", self.service, self.method)
    }
}
