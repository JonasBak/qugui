use super::config::*;
use super::handler::*;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, RadioButton};
use std::collections::HashMap;
use std::sync::mpsc;

#[derive(Debug)]
pub enum MsgGui {
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
    SetActive {
        node: usize,
        active: bool,
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

pub fn setup_gui(
    tx: mpsc::Sender<MsgHandler>,
    grx: glib::Receiver<MsgGui>,
    config: &Config,
    app: &Application,
) {
    let window = ApplicationWindow::new(app);
    window.set_title(&config.title);
    window.set_default_size(config.width.unwrap_or(600), config.height.unwrap_or(600));

    let mut containers = HashMap::new();
    let mut conditionals = HashMap::new();

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
                if btn.active_when.is_some() {
                    conditionals.insert(i, button.clone().upcast::<gtk::Widget>());
                }
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
            Node::Input(inp) => {
                let input = gtk::Entry::new();
                tx.send(MsgHandler::Var {
                    variable: inp.variable.clone(),
                    value: "".to_string(),
                })
                .unwrap();
                let tx = tx.clone();
                let variable = inp.variable.clone();
                input.connect_changed(move |input| {
                    tx.send(MsgHandler::Var {
                        variable: variable.clone(),
                        value: input.get_buffer().get_text(),
                    })
                    .unwrap();
                });
                if inp.active_when.is_some() {
                    conditionals.insert(i, input.clone().upcast::<gtk::Widget>());
                }

                (input.upcast::<gtk::Widget>(), &inp.placement)
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
            MsgGui::SetActive { node, active } => {
                if let Some(node) = conditionals.get(&node) {
                    node.set_sensitive(active);
                } else {
                    warn!(
                        "could not find node with index {} in conditionals map",
                        node
                    );
                }
            }
        }
        glib::Continue(true)
    });

    window.show_all();
    tx.send(MsgHandler::Initialize).unwrap();
}
