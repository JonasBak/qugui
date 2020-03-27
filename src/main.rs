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
    Action(usize),
    Var(String, String),
}

#[derive(Debug)]
enum MsgGui {
    Show(String, String),
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
    let mut group: Option<RadioButton> = None;
    for (value, label) in btns.iter() {
        let button = RadioButton::new_with_label(label);
        let tx = tx.clone();
        let value = value.clone().to_owned();
        let variable = var.clone();
        button.connect_toggled(move |btn| {
            if btn.get_active() {
                tx.send(MsgHandler::Var(variable.clone(), value.clone()))
                    .unwrap();
            }
        });
        container.pack_start(&button, false, false, 0);
        if let Some(group) = group.clone() {
            button.join_group(Some(&group));
        } else {
            group = Some(button.clone());
        }
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

    grx.attach(None, move |msg| {
        debug!("handler->gui: {:?}", msg);
        match msg {
            MsgGui::Show(name, text) => {
                if let Some(container) = containers.get(&name) {
                    container
                        .get_children()
                        .iter()
                        .for_each(|w| container.remove(w));
                    let label = gtk::Label::new(Some(&text));
                    container.add(&label);
                    container.show_all();
                } else {
                    warn!("could not find container with name {}", name);
                }
            }
        }
        glib::Continue(true)
    });

    window.show_all();
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
        MsgHandler::Var(var, value) => {
            vars.insert(var.clone(), value.clone());
            env::set_var(var, value);
            None
        }
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
                    last_out = child.stdout.take();
                }
                Action::Show { container } => {
                    if let Some(mut stdout) = last_out.take() {
                        let mut string = String::new();
                        stdout.read_to_string(&mut string).unwrap();
                        gtx.send(MsgGui::Show(container.clone(), string)).unwrap();
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
                        vars.insert(name.clone(), string.clone());
                        env::set_var(name, string);
                    } else {
                        warn!("can't show output, no stdout saved");
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
