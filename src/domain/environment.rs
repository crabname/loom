use super::variable::{default_variables, Variable};

#[derive(Debug, Clone)]
pub struct Environment {
    pub name: String,
    pub variables: Vec<Variable>,
}

impl Environment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            variables: default_variables(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentScope {
    Workspace,
    Collection(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvironmentRef {
    pub scope: EnvironmentScope,
    pub index: usize,
}
