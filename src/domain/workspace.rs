use super::{Collection, Environment, Variable};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub name: String,
    pub variables: Vec<Variable>,
    pub environments: Vec<Environment>,
    pub collections: Vec<Collection>,
}
