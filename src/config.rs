use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

pub type Condition = HashMap<String, String>;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    Run { command: Vec<String> },
    Show { container: String },
    Var { name: String, value: Option<String> },
    Options { variable: String, container: String },
    Image { variable: String, container: String },
}

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
    pub active_when: Option<Condition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RadioButtonsNode {
    pub variable: String,
    pub options: HashMap<String, String>,
    pub placement: Placement,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InputNode {
    pub variable: String,
    pub placement: Placement,
    pub active_when: Option<Condition>,
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
    RadioButtons(RadioButtonsNode),
    Container(ContainerNode),
    Input(InputNode),
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
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub nodes: Vec<Node>,
    pub layout: ConfigLayout,
    pub initialize: Option<Vec<Action>>,
}

pub fn read_config(filename: &String) -> Result<Config, serde_yaml::Error> {
    debug!("reading config from: {}", filename);

    let config: Config =
        serde_yaml::from_str(&fs::read_to_string(filename).expect("could not read config file"))?;

    debug!("using config:\n{:?}", config);

    Ok(config)
}
