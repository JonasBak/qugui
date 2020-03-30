use super::config::*;
use super::gui::*;
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::prelude::*;
use std::process::{Command, Stdio};

type Vars = HashMap<String, String>;

#[derive(Debug)]
pub enum MsgHandler {
    Initialize,
    Action(usize),
    Var { variable: String, value: String },
}

fn check_condition(condition: &Condition, vars: &Vars) -> bool {
    for (var, value) in condition.iter() {
        let (var, negate) = if var.ends_with("!") {
            (&var[..var.len() - 1], true)
        } else {
            (&var[..], false)
        };
        if match vars.get(var) {
            Some(set_value) => (value != set_value) ^ negate,
            None => !negate,
        } {
            return false;
        };
    }
    true
}

pub fn map_conditionals(config: &Config) -> HashMap<String, Vec<usize>> {
    let mut conditionals: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, node) in config.nodes.iter().enumerate() {
        let condition = match node {
            Node::Button(btn) => btn.active_when.as_ref(),
            Node::RadioButtons(_) => None,
            Node::Container(_) => None,
            Node::Input(inp) => inp.active_when.as_ref(),
        };
        if let Some(condition) = condition {
            for (var, _) in condition.iter() {
                let mut var = var.clone();
                if var.ends_with("!") {
                    var.pop();
                };
                let mut v = match conditionals.remove(&var) {
                    Some(v) => v,
                    None => vec![],
                };
                v.push(i);
                conditionals.insert(var, v);
            }
        }
    }
    conditionals
}

pub fn handle_msg(
    config: &Config,
    vars: &mut Vars,
    conditionals: &HashMap<String, Vec<usize>>,
    msg: MsgHandler,
    gtx: glib::Sender<MsgGui>,
) {
    debug!("gui->handler: {:?}", msg);
    let mut conditionals_set = HashSet::new();
    let actions = match msg {
        MsgHandler::Action(i) => match &config.nodes[i] {
            Node::Button(btn) => Some(&btn.on_click),
            Node::RadioButtons(_) => None,
            Node::Container(_) => None,
            Node::Input(_) => None,
        },
        MsgHandler::Var { variable, value } => {
            if let Some(nodes) = conditionals.get(&variable) {
                nodes.iter().for_each(|i| {
                    conditionals_set.insert(i.clone());
                });
            }
            vars.insert(variable.clone(), value.clone());
            env::set_var(variable, value);
            None
        }
        MsgHandler::Initialize => config.initialize.as_ref(),
    };
    if let Some(actions) = actions {
        let mut last_out = None;
        for action in actions.iter() {
            debug!("running action {:?}", action);
            match action {
                Action::Run { command } => {
                    let child = Command::new(&command[0])
                        .args(command.iter().skip(1).map(|arg| {
                            let mut arg = arg.clone();
                            for (key, value) in vars.iter() {
                                arg = arg.replace(key, value);
                            }
                            arg
                        }))
                        .stdin(match last_out.take() {
                            Some(child_stdout) => Stdio::from(child_stdout),
                            None => Stdio::piped(),
                        })
                        .stdout(Stdio::piped())
                        .spawn();
                    let mut child = match child {
                        Ok(child) => child,
                        Err(_) => {
                            error!("failed to start command {:?}", command);
                            break;
                        }
                    };
                    match child.wait() {
                        Ok(status) => {
                            if !status.success() {
                                error!(
                                    "command {:?} failed to run with status {}",
                                    command,
                                    status.code().unwrap_or(-1)
                                );
                            }
                        }
                        Err(_) => {
                            error!("failed to start command {:?}", command);
                            break;
                        }
                    };
                    last_out = child.stdout.take();
                }
                Action::Show { container } => {
                    if let Some(mut stdout) = last_out.take() {
                        let mut text = String::new();
                        stdout.read_to_string(&mut text).unwrap();
                        gtx.send(MsgGui::Show {
                            container: container.clone(),
                            text,
                        })
                        .unwrap();
                    } else {
                        warn!("can't show output, no stdout saved");
                    }
                }
                Action::Var { name, value } => {
                    if let Some(nodes) = conditionals.get(name) {
                        nodes.iter().for_each(|i| {
                            conditionals_set.insert(i.clone());
                        });
                    }
                    if let Some(value) = value {
                        vars.insert(name.clone(), value.clone());
                        env::set_var(name, value);
                    } else if let Some(mut stdout) = last_out.take() {
                        let mut string = String::new();
                        stdout.read_to_string(&mut string).unwrap();
                        if string.ends_with("\n") {
                            string.pop();
                        }
                        vars.insert(name.clone(), string.clone());
                        env::set_var(name, string);
                    } else {
                        warn!("can't show output, no stdout saved");
                    }
                }
                Action::Options {
                    variable,
                    container,
                } => {
                    if let Some(mut stdout) = last_out.take() {
                        let mut string = String::new();
                        stdout.read_to_string(&mut string).unwrap();
                        let lines = string.lines();
                        gtx.send(MsgGui::Options {
                            container: container.clone(),
                            variable: variable.to_owned(),
                            options: lines.map(|a| (a.to_string(), a.to_string())).collect(),
                        })
                        .unwrap();
                    } else {
                        warn!("can't create options, no stdout saved");
                    }
                }
                Action::Image {
                    variable,
                    container,
                } => {
                    if let Some(value) = vars.get(variable) {
                        gtx.send(MsgGui::Image {
                            container: container.clone(),
                            filename: value.clone(),
                        })
                        .unwrap();
                    } else {
                        warn!("variable {} not set", variable);
                    }
                }
            }
        }
    }
    for i in conditionals_set.into_iter() {
        let condition = match &config.nodes[i] {
            Node::Button(btn) => btn.active_when.as_ref(),
            Node::RadioButtons(_) => None,
            Node::Container(_) => None,
            Node::Input(inp) => inp.active_when.as_ref(),
        };
        if let Some(condition) = condition {
            gtx.send(MsgGui::SetActive {
                node: i,
                active: check_condition(condition, vars),
            })
            .unwrap();
        }
    }
}
