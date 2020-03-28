extern crate gdk_pixbuf;
extern crate gio;
extern crate glib;
extern crate gtk;
extern crate serde;
#[macro_use]
extern crate log;
extern crate env_logger;

pub mod config;
pub mod gui;
pub mod handler;

use gio::prelude::*;
use gtk::Application;
use std::collections::HashMap;
use std::env;
use std::sync::mpsc;
use std::thread;

use config::*;
use gui::*;
use handler::*;

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
