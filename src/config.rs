use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    Run { command: Vec<String> },
    Show { container: String },
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

pub fn read_config() -> Result<Config, serde_yaml::Error> {
    let config: Config = serde_yaml::from_str(
        r#"
---
title: Example
layout:
  Vertical:
    spacing: 1
nodes:
- type: Button
  text: Button 1
  on_click:
  - type: Run
    command: ["git", "status", "-s"]
  - type: Run
    command: ["grep", "rs$"]
  - type: Show
    container: container01
  placement:
    spacing: 10
- type: Button
  text: Button 2
  on_click:
  - type: Run
    command: ["date"]
  - type: Show
    container: container01
  placement:
    spacing: 10
- type: Container
  name: container01
  placement:
    spacing: 10
    "#,
    )?;

    debug!("Using config:\n{:?}", config);

    Ok(config)
}
