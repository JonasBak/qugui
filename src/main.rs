extern crate gdk_pixbuf;
extern crate gio;
extern crate glib;
extern crate gtk;
extern crate serde;
#[macro_use]
extern crate log;
extern crate env_logger;

pub mod config;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, RadioButton};
use std::collections::HashMap;
use std::env;
use std::io::prelude::*;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use config::*;

#[derive(Debug)]
enum MsgHandler {
    Initialize,
    Action(usize),
    Var { variable: String, value: String },
}

#[derive(Debug)]
enum MsgGui {
    Show {
        container: String,
        text: String,
    },
    Options {
        container: String,
        variable: String,
        options: Vec<(String, String)>,
    },
    Image {
        container: String,
        filename: String,
    },
}

enum Layout {
    Box(gtk::Box),
    Grid(gtk::Grid),
}

fn create_radio_buttons(
    btns: Vec<(&String, &String)>,
    var: String,
    tx: mpsc::Sender<MsgHandler>,
) -> gtk::Widget {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let mut group: Option<(String, RadioButton)> = None;
    for (value, label) in btns.iter() {
        let button = RadioButton::new_with_label(label);
        let tx = tx.clone();
        let value_clone = value.clone().to_owned();
        let var = var.clone();
        button.connect_toggled(move |btn| {
            if btn.get_active() {
                tx.send(MsgHandler::Var {
                    variable: var.clone(),
                    value: value_clone.clone(),
                })
                .unwrap();
            }
        });
        container.pack_start(&button, false, false, 0);
        if let Some((_, group)) = group.clone() {
            button.join_group(Some(&group));
        } else {
            group = Some((value.clone().to_owned(), button.clone()));
        }
    }
    if let Some((value, _)) = group.clone() {
        tx.send(MsgHandler::Var {
            variable: var,
            value: value.clone(),
        })
        .unwrap();
    }
    container.upcast::<gtk::Widget>()
}

fn setup_gui(
    tx: mpsc::Sender<MsgHandler>,
    grx: glib::Receiver<MsgGui>,
    config: &Config,
    app: &Application,
) {
    let window = ApplicationWindow::new(app);
    window.set_title(&config.title);
    window.set_default_size(600, 600);

    let mut containers = HashMap::new();

    let layout = match config.layout {
        ConfigLayout::Vertical { spacing } => Layout::Box(gtk::Box::new(
            gtk::Orientation::Vertical,
            spacing.unwrap_or(0),
        )),
        ConfigLayout::Horizontal { spacing } => Layout::Box(gtk::Box::new(
            gtk::Orientation::Horizontal,
            spacing.unwrap_or(0),
        )),
        ConfigLayout::Grid => {
            let grid = gtk::Grid::new();
            grid.set_row_homogeneous(true);
            grid.set_column_homogeneous(true);
            Layout::Grid(grid)
        }
    };
    for (i, node) in config.nodes.iter().enumerate() {
        let (n, p) = match node {
            Node::Button(btn) => {
                let button = Button::new_with_label(&btn.text);
                let tx = tx.clone();
                button.connect_clicked(move |_| {
                    tx.send(MsgHandler::Action(i)).unwrap();
                });
                (button.upcast::<gtk::Widget>(), &btn.placement)
            }
            Node::RadioButtons(btns) => {
                let container = create_radio_buttons(
                    btns.options.iter().collect(),
                    btns.variable.clone(),
                    tx.clone(),
                );
                (container, &btns.placement)
            }
            Node::Container(cont) => {
                let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let w = container.clone().upcast::<gtk::Widget>();
                containers.insert(cont.name.clone(), container);
                (w, &cont.placement)
            }
        };
        match &layout {
            Layout::Box(container) => {
                container.pack_start(&n, false, false, p.spacing.unwrap_or(0))
            }
            Layout::Grid(container) => container.attach(
                &n,
                p.x.unwrap_or(0),
                p.y.unwrap_or(0),
                p.w.unwrap_or(1),
                p.h.unwrap_or(1),
            ),
        };
    }

    match layout {
        Layout::Box(container) => window.add(&container.upcast::<gtk::Widget>()),
        Layout::Grid(container) => window.add(&container.upcast::<gtk::Widget>()),
    };

    let tx2 = tx.clone();
    grx.attach(None, move |msg| {
        debug!("handler->gui: {:?}", msg);
        match msg {
            MsgGui::Show { container, text } => {
                if let Some(container) = containers.get(&container) {
                    container
                        .get_children()
                        .iter()
                        .for_each(|w| container.remove(w));
                    let label = gtk::Label::new(Some(&text));
                    container.add(&label);
                    container.show_all();
                } else {
                    warn!("could not find container with name {}", container);
                }
            }
            MsgGui::Options {
                container,
                variable,
                options,
            } => {
                if let Some(container) = containers.get(&container) {
                    container
                        .get_children()
                        .iter()
                        .for_each(|w| container.remove(w));
                    let buttons = create_radio_buttons(
                        options.iter().map(|(a, b)| (a, b)).collect(),
                        variable,
                        tx2.clone(),
                    );
                    container.add(&buttons);
                    container.show_all();
                } else {
                    warn!("could not find container with name {}", container);
                }
            }
            MsgGui::Image {
                container,
                filename,
            } => {
                if let Some(container) = containers.get(&container) {
                    container
                        .get_children()
                        .iter()
                        .for_each(|w| container.remove(w));
                    let pixbuf = gdk_pixbuf::Pixbuf::new_from_file_at_scale(
                        &filename,
                        container.get_allocated_width(),
                        container.get_allocated_height(),
                        true,
                    )
                    .unwrap();
                    let image = gtk::Image::new_from_pixbuf(Some(&pixbuf));
                    container.add(&image);
                    container.show_all();
                } else {
                    warn!("could not find container with name {}", container);
                }
            }
        }
        glib::Continue(true)
    });

    window.show_all();
    tx.send(MsgHandler::Initialize).unwrap();
}

fn handle_msg(
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
            match action {
                Action::Run { command } => {
                    let mut child = Command::new(&command[0])
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
                        .spawn()
                        .unwrap();
                    child.wait().unwrap();
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

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let config = read_config(&args[1]).expect("could not parse config file");

    let application = Application::new(Some("com.github.jonasbak.qugui"), Default::default())
        .expect("failed to initialize GTK application");

    application.connect_activate(move |app| {
        let (tx, rx) = mpsc::channel::<MsgHandler>();
        let (gtx, grx) = glib::MainContext::channel::<MsgGui>(glib::PRIORITY_DEFAULT);

        let config_clone = config.clone();
        thread::spawn(move || {
            let mut vars = HashMap::new();
            rx.iter()
                .for_each(|msg| handle_msg(&config_clone, &mut vars, msg, gtx.clone()));
        });

        setup_gui(tx.clone(), grx, &config, app);
    });

    application.run(&[]);
}
