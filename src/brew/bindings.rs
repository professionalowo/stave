use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrewList {
    #[serde(rename = "formulae", alias = "formulas", default)]
    pub formulas: Vec<Formula>,
    #[serde(default)]
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

pub type Info = Vec<InfoEntry>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InfoEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub caveats: Option<String>,
    #[serde(default)]
    pub installed: Vec<InstalledInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstalledInfo {
    #[serde(default)]
    pub version: Option<String>,
}
