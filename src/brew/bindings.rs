use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrewList {
    pub formulas: Vec<Formula>,
    pub casks: Vec<Cask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
    pub name: String,
    pub versions: Vec<String>,
    pub linked_version: Option<String>,
    pub optlinked_version: Option<String>,
    pub pinned_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cask {
    pub token: String,
    pub versions: Vec<String>,
    pub pinned_version: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {}
