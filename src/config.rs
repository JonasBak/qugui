use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    Run { command: Vec<String> },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ButtonNode {
    pub text: String,
    pub on_click: Vec<Action>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Node {
    Button(ButtonNode),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub title: String,
    pub nodes: Vec<Node>,
}

pub fn read_config() -> Result<Config, serde_yaml::Error> {
    let config: Config = serde_yaml::from_str(
        r#"
---
title: Example
nodes:
- type: Button
  text: Button 1
  on_click:
  - type: Run
    command: ["git", "status", "-s"]
- type: Button
  text: Button 2
  on_click:
  - type: Run
    command: ["date"]
    "#,
    )?;

    println!("{:?}", config);

    Ok(config)
}
