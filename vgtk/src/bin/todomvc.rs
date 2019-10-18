#![recursion_limit = "16384"]

extern crate gio;
extern crate glib;
extern crate gtk;
extern crate strum;
#[macro_use]
extern crate strum_macros;

#[macro_use]
extern crate vgtk;

use std::fmt::{Debug, Display};

use strum::IntoEnumIterator;

use gio::ApplicationFlags;
use gtk::prelude::*;
use gtk::*;
use vgtk::{ext::*, go, vnode::VNode, Callback, Component};

#[derive(Clone, Debug, Default)]
struct Radio<Enum: Unpin> {
    active: Enum,
    on_changed: Option<Callback<Enum>>,
}

#[derive(Clone, Debug)]
enum RadioMsg<Enum: Unpin> {
    Selected(Enum),
}

impl<Enum, I> Component for Radio<Enum>
where
    Enum: 'static
        + IntoEnumIterator<Iterator = I>
        + Display
        + PartialEq
        + Debug
        + Default
        + Copy
        + Send
        + Unpin,
    I: Iterator<Item = Enum>,
{
    type Message = RadioMsg<Enum>;
    type Properties = Self;

    fn create(props: Self::Properties) -> Self {
        props
    }

    fn change(&mut self, props: Self::Properties) -> bool {
        *self = props;
        true
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        match msg {
            RadioMsg::Selected(selected) => {
                self.active = selected;
                if let Some(ref callback) = self.on_changed {
                    callback.send(self.active.clone());
                }
            }
        }
        true
    }

    fn view(&self) -> VNode<Radio<Enum>> {
        gtk! {
            <Box orientation={Orientation::Horizontal} spacing=10>
                { Enum::iter().map(|label| {
                    gtk!{
                        <ToggleButton label={label.to_string()} active={label == self.active}
                                      on toggled=|_| {RadioMsg::Selected(label)}/>
                    }
                }) }
            </Box>
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Display, EnumIter)]
#[repr(u32)]
enum Filter {
    All,
    Active,
    Completed,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::All
    }
}

#[derive(Clone, Default, Debug)]
struct Item {
    label: String,
    done: bool,
}

impl Item {
    fn new<S: Into<String>>(label: S) -> Self {
        Item {
            label: label.into(),
            done: false,
        }
    }
}

#[derive(Clone, Debug)]
struct Model {
    items: Vec<Item>,
    filter: Filter,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            items: ["foo", "bar"].iter().cloned().map(Item::new).collect(),
            filter: Filter::All,
        }
    }
}

impl Model {
    fn filter(&self, filter: Filter) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(move |item| match filter {
            Filter::All => true,
            Filter::Active => !item.done,
            Filter::Completed => item.done,
        })
    }

    fn left_label(&self) -> String {
        let left = self.filter(Filter::Active).count();
        match left {
            1 => String::from("1 item left"),
            left => format!("{} items left", left),
        }
    }
}

#[derive(Clone, Debug)]
enum Msg {
    Add { item: String },
    Remove { index: usize },
    Toggle { index: usize },
    Filter { filter: Filter },
    ToggleAll,
    ClearCompleted,
    Exit,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn update(&mut self, msg: Self::Message) -> bool {
        let left = self.filter(Filter::Active).count();
        match msg {
            Msg::Add { item } => {
                self.items.push(Item::new(item));
            }
            Msg::Remove { index } => {
                self.items.remove(index);
            }
            Msg::Toggle { index } => self.items[index].done = !self.items[index].done,
            Msg::Filter { filter } => self.filter = filter,
            Msg::ToggleAll if left > 0 => self.items.iter_mut().for_each(|item| item.done = true),
            Msg::ToggleAll => self.items.iter_mut().for_each(|item| item.done = false),
            Msg::ClearCompleted => self.items.retain(|item| !item.done),
            Msg::Exit => {
                vgtk::main_quit(0);
            }
        }
        true
    }

    fn view(&self) -> VNode<Model> {
        gtk! {
            <Window default_width=800 default_height=480 border_width=20u32 on destroy=|_| {Msg::Exit}>
                <HeaderBar title="TodoMVC" subtitle="wtf do we do now" show_close_button=true />
                <Box spacing=10 orientation={Orientation::Vertical}>
                    <Box spacing=10 orientation={Orientation::Horizontal} Box::expand=false>
                        <Button image="edit-select-all" relief={ReliefStyle::Half}
                                always_show_image=true on clicked=|_| {Msg::ToggleAll}/>
                        <Entry placeholder_text="What needs to be done?"
                               Box::expand=true Box::fill=true
                               on activate=|entry| {
                                   let label = entry.get_text().map(|s| s.to_string()).unwrap_or_default();
                                   entry.select_region(0, label.len() as i32);
                                   Msg::Add {
                                       item: label
                                   }
                               } />
                    </Box>
                    <ScrolledWindow Box::expand=true Box::fill=true>
                        <ListBox selection_mode={SelectionMode::None}>
                            {
                                self.filter(self.filter).enumerate()
                                    .map(|(index, item)| render_item(index, item))
                            }
                        </ListBox>
                    </ScrolledWindow>
                    <Box spacing=10 orientation={Orientation::Horizontal} Box::expand=false>
                        <Label label={self.left_label()}/>
                        <@Radio<Filter> Box::expand=true active={self.filter} on_changed={|filter| Msg::Filter { filter }} />
                        {
                            if self.filter(Filter::Completed).count() > 0 {
                                (gtk!{
                                     <Button label="Clear completed" Box::pack_type={PackType::End}
                                             on clicked=|_| {Msg::ClearCompleted}/>
                                }).into_iter()
                            } else {
                                VNode::empty()
                            }
                        }
                    </Box>
                </Box>
            </Window>
        }
    }
}

fn render_item(index: usize, item: &Item) -> VNode<Model> {
    let label = if item.done {
        format!(
            "<span strikethrough=\"true\" alpha=\"50%\">{}</span>",
            item.label
        )
    } else {
        item.label.clone()
    };
    gtk! {
        <ListBoxRow>
            <Box spacing=10 orientation={Orientation::Horizontal}>
                <CheckButton active={item.done} on toggled=|_| {Msg::Toggle { index }} />
                <Label label=label use_markup=true Box::fill=true />
                <Button Box::pack_type={PackType::End} relief={ReliefStyle::None}
                        always_show_image=true image="edit-delete"
                        on clicked=|_| {Msg::Remove { index }} />
            </Box>
        </ListBoxRow>
    }
}

fn main() {
    std::process::exit(go::<Model>("camp.lol.updog", ApplicationFlags::empty()));
}
