use super::config::*;
use super::gui::*;
use std::collections::HashMap;
use std::env;
use std::io::prelude::*;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub enum MsgHandler {
    Initialize,
    Action(usize),
    Var { variable: String, value: String },
}

pub fn handle_msg(
    config: &Config,
    vars: &mut HashMap<String, String>,
    msg: MsgHandler,
    gtx: glib::Sender<MsgGui>,
) {
    debug!("gui->handler: {:?}", msg);
    let actions = match msg {
        MsgHandler::Action(i) => match &config.nodes[i] {
            Node::Button(btn) => Some(&btn.on_click),
            Node::RadioButtons(_) => None,
            Node::Container(_) => None,
            Node::Input(_) => None,
        },
        MsgHandler::Var { variable, value } => {
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
}
