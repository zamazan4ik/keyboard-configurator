use cascade::cascade;
use gtk::{
    glib::{self, clone, subclass::Signal},
    pango,
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;
use std::cell::{Cell, RefCell};

use super::{picker_group_box::PickerGroupBox, PickerKey, SCANCODE_LABELS};
use backend::{is_qmk_basic, DerefCell, Keycode, Mods};

#[derive(Clone, Copy, PartialEq)]
enum Hold {
    Mods(Mods),
    Layer(u8),
}

impl Default for Hold {
    fn default() -> Self {
        Self::Mods(Mods::default())
    }
}

static MODIFIERS: &[&str] = &[
    "LEFT_SHIFT",
    "LEFT_CTRL",
    "LEFT_SUPER",
    "LEFT_ALT",
    "RIGHT_SHIFT",
    "RIGHT_CTRL",
    "RIGHT_SUPER",
    "RIGHT_ALT",
];
pub static LAYERS: &[&str] = &["LAYER_ACCESS_1", "FN", "LAYER_ACCESS_3", "LAYER_ACCESS_4"];

#[derive(Default)]
pub struct TapHoldInner {
    shift: Cell<bool>,
    hold: Cell<Hold>,
    keycode: RefCell<Option<String>>,
    mod_buttons: DerefCell<Vec<PickerKey>>,
    layer_buttons: DerefCell<Vec<PickerKey>>,
    picker_group_box: DerefCell<PickerGroupBox>,
}

#[glib::object_subclass]
impl ObjectSubclass for TapHoldInner {
    const NAME: &'static str = "S76KeyboardTapHold";
    type ParentType = gtk::Box;
    type Type = TapHold;
}

impl ObjectImpl for TapHoldInner {
    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder(
                "select",
                &[Keycode::static_type().into()],
                glib::Type::UNIT.into(),
            )
            .build()]
        });
        SIGNALS.as_ref()
    }

    fn constructed(&self, widget: &Self::Type) {
        self.parent_constructed(widget);

        let picker_group_box = cascade! {
            PickerGroupBox::new("basics");
            ..set_sensitive(false);
            ..connect_key_pressed(clone!(@weak widget => move |name, _shift| {
                *widget.inner().keycode.borrow_mut() = Some(name);
                widget.update();
            }));
            // Correct?
            ..set_key_visibility(|name| is_qmk_basic(name));
        };

        let modifier_button_box = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 0);
        };
        let mut mod_buttons = Vec::new();
        for i in MODIFIERS {
            let label = SCANCODE_LABELS.get(*i).unwrap();
            let mod_ = Mods::from_mod_str(*i).unwrap();
            let button = cascade! {
                PickerKey::new(i, label, 2);
                ..connect_clicked_with_shift(clone!(@weak widget => move |_, shift| {
                    let mut new_mods = mod_;
                    if shift {
                        if let Hold::Mods(mods) = widget.inner().hold.get() {
                            new_mods = mods.toggle_mod(mod_);
                        }
                    }
                    widget.inner().hold.set(Hold::Mods(new_mods));
                    widget.update();
                }));
            };
            modifier_button_box.add(&button);
            mod_buttons.push(button);
        }
        self.mod_buttons.set(mod_buttons);

        let layer_button_box = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 0);
        };
        let mut layer_buttons = Vec::new();
        for (n, i) in LAYERS.iter().enumerate() {
            let label = SCANCODE_LABELS.get(*i).unwrap();
            let button = cascade! {
                PickerKey::new(i, label, 2);
                ..connect_clicked(clone!(@weak widget => move |_| {
                    widget.inner().hold.set(Hold::Layer(n as u8));
                    widget.update();

                }));
            };
            layer_button_box.add(&button);
            layer_buttons.push(button);
        }
        self.layer_buttons.set(layer_buttons);

        // TODO: select monifier/layer; multiple select; when both are selected, set keycode

        cascade! {
            widget;
            ..set_orientation(gtk::Orientation::Vertical);
            ..add(&cascade! {
                gtk::Label::new(Some("1. Select action(s) to use when the key is held."));
                ..set_attributes(Some(&cascade! {
                    pango::AttrList::new();
                    ..insert(pango::AttrInt::new_weight(pango::Weight::Bold));
                }));
                ..set_halign(gtk::Align::Start);
            });
            ..add(&modifier_button_box);
            ..add(&layer_button_box);
            ..add(&cascade! {
                gtk::Label::new(Some("2. Select an action to use when the key is tapped."));
                ..set_attributes(Some(&cascade! {
                    pango::AttrList::new();
                    ..insert(pango::AttrInt::new_weight(pango::Weight::Bold));
                }));
                ..set_halign(gtk::Align::Start);
            });
            ..add(&picker_group_box);
        };

        self.picker_group_box.set(picker_group_box);
    }
}

impl BoxImpl for TapHoldInner {}
impl WidgetImpl for TapHoldInner {}
impl ContainerImpl for TapHoldInner {}

glib::wrapper! {
    pub struct TapHold(ObjectSubclass<TapHoldInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl TapHold {
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    fn inner(&self) -> &TapHoldInner {
        TapHoldInner::from_instance(self)
    }

    fn update(&self) {
        let keycode = self.inner().keycode.borrow();
        let keycode = keycode.as_deref().unwrap_or("NONE");
        match self.inner().hold.get() {
            Hold::Mods(mods) => {
                if !mods.is_empty() {
                    self.emit_by_name::<()>("select", &[&Keycode::MT(mods, keycode.to_string())]);
                }
            }
            Hold::Layer(layer) => {
                self.emit_by_name::<()>("select", &[&Keycode::LT(layer, keycode.to_string())]);
            }
        }
    }

    pub fn connect_select<F: Fn(Keycode) + 'static>(&self, cb: F) -> glib::SignalHandlerId {
        self.connect_local("select", false, move |values| {
            cb(values[1].get::<Keycode>().unwrap());
            None
        })
    }

    pub(crate) fn set_selected(&self, scancode_names: Vec<Keycode>) {
        // XXX how to handle > 1?
        let (mods, layer, keycode) = if scancode_names.len() == 1 {
            match scancode_names.into_iter().next().unwrap() {
                Keycode::MT(mods, keycode) => (mods, None, Some(keycode)),
                Keycode::LT(layer, keycode) => (Mods::empty(), Some(layer), Some(keycode)),
                Keycode::Basic(..) => Default::default(),
            }
        } else {
            Default::default()
        };

        for i in self.inner().mod_buttons.iter() {
            let mod_ = Mods::from_mod_str(i.name()).unwrap();
            i.set_selected(
                mods.contains(mod_) && (mods.contains(Mods::RIGHT) == mod_.contains(Mods::RIGHT)),
            );
        }

        for (n, i) in self.inner().layer_buttons.iter().enumerate() {
            i.set_selected(Some(n as u8) == layer);
        }

        if let Some(keycode) = keycode.clone() {
            self.inner()
                .picker_group_box
                .set_selected(vec![Keycode::Basic(Mods::empty(), keycode)]);
        } else {
            self.inner().picker_group_box.set_selected(Vec::new());
        }

        self.inner().hold.set(if let Some(layer) = layer {
            Hold::Layer(layer)
        } else {
            Hold::Mods(mods)
        });
        *self.inner().keycode.borrow_mut() = keycode;

        self.invalidate_sensitivity();
    }

    pub fn set_shift(&self, shift: bool) {
        self.inner().shift.set(shift);
        self.invalidate_sensitivity();
    }

    fn invalidate_sensitivity(&self) {
        let shift = self.inner().shift.get();
        let hold = self.inner().hold.get();
        let hold_empty = hold == Hold::Mods(Mods::empty());
        let keycode = self.inner().keycode.borrow();

        for button in self.inner().layer_buttons.iter() {
            button.set_sensitive(if shift {
                hold == Hold::Mods(Mods::empty())
            } else {
                true
            });
        }

        for button in self.inner().mod_buttons.iter() {
            button.set_sensitive(if shift {
                match hold {
                    Hold::Mods(mods) => {
                        let right = button.name().starts_with("RIGHT");
                        mods.is_empty() || (right == mods.contains(Mods::RIGHT))
                    }
                    Hold::Layer(_) => false,
                }
            } else {
                true
            });
        }

        self.inner().picker_group_box.set_sensitive(if shift {
            !hold_empty && keycode.is_none()
        } else {
            !hold_empty
        });
    }
}
