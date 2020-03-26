extern crate gio;
extern crate gtk;
extern crate serde;

pub mod config;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button};
use std::process::Command;
use std::sync::mpsc;
use std::thread;

use config::*;

#[derive(Debug)]
enum Msg {
    Action(usize),
}

enum Layout {
    Box(gtk::Box),
    Grid(gtk::Grid),
}

fn setup_gui(tx: mpsc::Sender<Msg>, config: &Config, app: &Application) {
    let window = ApplicationWindow::new(app);
    window.set_title(&config.title);
    window.set_default_size(350, 70);

    let layout = match config.layout {
        ConfigLayout::Vertical { spacing } => Layout::Box(gtk::Box::new(
            gtk::Orientation::Vertical,
            spacing.unwrap_or(0),
        )),
        ConfigLayout::Horizontal { spacing } => Layout::Box(gtk::Box::new(
            gtk::Orientation::Horizontal,
            spacing.unwrap_or(0),
        )),
        ConfigLayout::Grid => Layout::Grid(gtk::Grid::new()),
    };
    for (i, node) in config.nodes.iter().enumerate() {
        let (n, p) = match node {
            Node::Button(btn) => {
                let button = Button::new_with_label(&btn.text);
                let tx = tx.clone();
                button.connect_clicked(move |_| {
                    tx.send(Msg::Action(i)).unwrap();
                });
                (button.upcast::<gtk::Widget>(), &btn.placement)
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

    window.show_all();
}

fn do_action(action: &Action) {
    match action {
        Action::Run { command } => {
            let output = Command::new(&command[0])
                .args(command.iter().skip(1))
                .output()
                .unwrap();
            if !output.status.success() {
                println!("Command executed with failing error code");
                return;
            }
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .for_each(|x| println!("{:?}", x));
        }
    }
}

fn handle_msg(config: &Config, msg: Msg) {
    println!("{:?}", msg);
    match msg {
        Msg::Action(i) => {
            let actions = match &config.nodes[i] {
                Node::Button(btn) => &btn.on_click,
            };
            for action in actions.iter() {
                do_action(action);
            }
        }
    };
}

fn main() {
    let config = read_config().expect("could not parse config file");

    let (tx, rx) = mpsc::channel::<Msg>();

    let config_clone = config.clone();
    thread::spawn(move || {
        let application = Application::new(Some("com.github.jonasbak.qugui"), Default::default())
            .expect("failed to initialize GTK application");
        application.connect_activate(move |app| setup_gui(tx.clone(), &config_clone, app));
        application.run(&[]);
    });
    rx.iter().for_each(|msg| handle_msg(&config, msg));
}
