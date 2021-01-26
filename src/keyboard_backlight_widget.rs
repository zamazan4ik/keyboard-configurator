// Intended for use in Gnome Control Center's Keyboard panel

use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use std::rc::Rc;

use crate::daemon::{Daemon, DaemonS76Power};
use crate::keyboard_color_button::KeyboardColorButton;

pub fn keyboard_backlight_widget() -> gtk::Widget {
    let stack = cascade! {
        gtk::Stack::new();
        ..get_style_context().add_class("frame");
        ..set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    };

    let stack_switcher = cascade! {
        gtk::StackSwitcher::new();
        ..set_stack(Some(&stack));
    };

    let vbox = cascade! {
        gtk::Box::new(gtk::Orientation::Vertical, 12);
        ..add(&stack_switcher);
        ..add(&stack);
    };

    if let Err(err) = add_boards(&stack) {
        eprintln!("Failed to get keyboards: {}", err);
    }

    vbox.upcast()
}

fn add_boards(stack: &gtk::Stack) -> Result<(), String> {
    let daemon = Rc::new(DaemonS76Power::new()?);
    let boards = daemon.boards()?;

    for (i, title) in boards.iter().enumerate() {
        stack.add_titled(&page(&daemon, i), &title, &title);
    }

    Ok(())
}

fn page(daemon: &Rc<DaemonS76Power>, daemon_board: usize) -> gtk::Widget {
    let max_brightness = daemon.max_brightness(daemon_board).unwrap_or(100) as f64;
    let brightness = daemon.brightness(daemon_board).unwrap_or(0) as f64;
    let brightness_scale = cascade! {
        gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., max_brightness, 1.);
        ..set_hexpand(true);
        ..set_draw_value(false);
        ..set_value(brightness);
        ..connect_change_value(clone!(@weak daemon => @default-panic, move |_scale, _, value| {
            if let Err(err) = daemon.set_brightness(daemon_board, value as i32) {
                eprintln!("Failed to set keyboard brightness: {}", err);
            }
            Inhibit(false)
        }));
    };

    // TODO detect when brightness changed in daemon

    let button = KeyboardColorButton::new(daemon.clone(), daemon_board);

    let listbox = cascade! {
        gtk::ListBox::new();
        ..set_header_func(Some(Box::new(|row, before| {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            row.set_header(before.and(Some(&separator)));
        })));
        ..add(&row("Brightness", &brightness_scale, true));
        ..add(&row("Color", &button, false));
    };

    listbox.upcast()
}

fn row<W: IsA<gtk::Widget>>(text: &str, widget: &W, expand: bool) -> gtk::ListBoxRow {
    let label = cascade! {
        gtk::Label::new(Some(text));
        ..set_justify(gtk::Justification::Left);
    };

    let hbox = cascade! {
        gtk::Box::new(gtk::Orientation::Horizontal, 24);
        ..set_hexpand(true);
        ..set_vexpand(true);
        ..pack_start(&label, false, false, 0);
        ..pack_end(widget, expand, expand, 0);
    };

    let list_box_row = cascade! {
        gtk::ListBoxRow::new();
        ..set_selectable(false);
        ..set_activatable(false);
        ..set_margin_top(12);
        ..set_margin_bottom(12);
        ..set_margin_start(12);
        ..set_margin_end(12);
        ..add(&hbox);
    };

    list_box_row
}
