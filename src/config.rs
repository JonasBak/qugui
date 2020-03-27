use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    Run { command: Vec<String> },
    Show { container: String },
    Var { name: String, value: Option<String> },
}

// #[serde(tag = "type")]
// Sould maybe be enum, but it's gonna be bad either way
#[derive(Debug, Clone, Deserialize)]
pub struct Placement {
    pub spacing: Option<u32>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub w: Option<i32>,
    pub h: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ButtonNode {
    pub text: String,
    pub on_click: Vec<Action>,
    pub placement: Placement,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContainerNode {
    pub name: String,
    pub placement: Placement,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Node {
    Button(ButtonNode),
    Container(ContainerNode),
}

// TODO check "bug" where spacing is left out and program panics
#[derive(Debug, Clone, Deserialize)]
pub enum ConfigLayout {
    Vertical { spacing: Option<i32> },
    Horizontal { spacing: Option<i32> },
    Grid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub title: String,
    pub nodes: Vec<Node>,
    pub layout: ConfigLayout,
}

pub fn read_config(filename: &String) -> Result<Config, serde_yaml::Error> {
    debug!("reading config from: {}", filename);

    let config: Config =
        serde_yaml::from_str(&fs::read_to_string(filename).expect("could not read config file"))?;

    debug!("using config:\n{:?}", config);

    Ok(config)
}
