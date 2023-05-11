use std::fmt::Display;
use std::str::FromStr;

use bevy::prelude::*;
use bevy::reflect::{
    Array, DynamicArray, DynamicEnum, DynamicInfo, DynamicList, DynamicMap, DynamicStruct,
    DynamicTuple, DynamicTupleStruct, DynamicVariant, Enum, EnumInfo, List, Map, Tuple, TypeInfo,
    VariantInfo, VariantType,
};
use bevy::utils::HashMap;
use bevy_egui::egui::{self, InnerResponse, ScrollArea, Ui};

use super::{Popup, Popups, ReprEditors};

pub enum EditorState {
    TextEdit { temp_value: String },
    Composite,
}

impl EditorState {
    pub fn text_edit(&mut self) -> &mut String {
        match self {
            Self::TextEdit { temp_value } => temp_value,
            _ => panic!(),
        }
    }

    pub fn composite(&mut self) {
        match self {
            Self::Composite => (),
            _ => panic!(),
        }
    }
}

#[derive(Default)]
pub struct Ctor {
    value: Option<Box<dyn Reflect>>,
    fresh: bool,
}

impl Ctor {
    pub fn start(&mut self, value: Box<dyn Reflect>) {
        self.value = Some(value);
        self.fresh = true;
    }

    pub fn poll(
        &mut self,
        ui: &mut Ui,
        world: &mut World,
        editors: &ReprEditors,
        states: &mut EditorStates,
    ) -> Option<Box<dyn Reflect>> {
        if self.value.is_some() {
            egui::Window::new("Constructor")
                .id(ui.auto_id_with("ctor"))
                .title_bar(false)
                .show(ui.ctx(), |ui| {
                    let value = self.value.as_mut().unwrap();

                    ui.vertical_centered(|ui| ui.heading("Constructor"));

                    let editor = editors.get(value.type_name());
                    ui.push_id(0, |ui| {
                        if self.fresh {
                            states.remove(ui.id());
                        }
                        editor(ui, &mut **value, world, editors, states)
                    });
                    ui.vertical_centered(|ui| {
                        if ui.button("apply").clicked() {
                            self.value.take()
                        } else {
                            if self.fresh {
                                self.fresh = false;
                            }
                            None
                        }
                    })
                })?
                .inner?
                .inner
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct Ctors {
    ctors: Vec<Ctor>,
}

impl Ctors {
    pub fn first(&mut self) -> &mut Ctor {
        if !self.ctors.is_empty() {
            &mut self.ctors[0]
        } else {
            self.ctors.insert(0, Ctor::default());
            self.ctors.first_mut().unwrap()
        }
    }

    pub fn nth(&mut self, n: usize) -> &mut Ctor {
        if self.ctors.len() > n {
            &mut self.ctors[n]
        } else {
            self.ctors.resize_with(n + 1, Ctor::default);
            &mut self.ctors[n]
        }
    }
}

#[derive(Default, Resource)]
pub struct EditorStates {
    state: HashMap<egui::Id, EditorState>,
    ctors: HashMap<egui::Id, Ctors>,
}

impl EditorStates {
    pub fn get(&mut self, id: egui::Id) -> Option<&mut EditorState> {
        self.state.get_mut(&id)
    }

    pub fn get_or(
        &mut self,
        id: egui::Id,
        default: impl FnOnce() -> EditorState,
    ) -> &mut EditorState {
        self.state.entry(id).or_insert_with(default)
    }

    pub fn init(
        &mut self,
        id: egui::Id,
        default: impl FnOnce() -> EditorState,
    ) -> (bool, &mut EditorState) {
        match self.state.contains_key(&id) {
            true => (false, self.state.get_mut(&id).unwrap()),
            false => {
                self.state.insert(id, default());
                (true, self.state.get_mut(&id).unwrap())
            }
        }
    }

    pub fn insert(&mut self, id: egui::Id, state: EditorState) {
        self.state.insert(id, state);
    }

    pub fn remove(&mut self, id: egui::Id) -> Option<EditorState> {
        self.state.remove(&id)
    }

    pub fn ctors<R>(
        &mut self,
        id: egui::Id,
        f: impl FnOnce(&mut EditorStates, &mut Ctors) -> R,
    ) -> R {
        let mut ctors = self.ctors.remove(&id).unwrap_or_default();
        let res = f(self, &mut ctors);
        self.ctors.insert(id, ctors);
        res
    }
}

pub trait FieldAccess {
    fn field_len(&self) -> usize;

    fn field(&mut self, index: usize) -> &mut dyn Reflect;

    fn name(&self, index: usize) -> Option<&str>;

    fn type_name(&self) -> &str;
}

impl FieldAccess for &mut dyn Struct {
    fn field_len(&self) -> usize {
        Struct::field_len(*self)
    }

    fn field(&mut self, index: usize) -> &mut dyn Reflect {
        self.field_at_mut(index).unwrap()
    }

    fn name(&self, index: usize) -> Option<&str> {
        Some(self.name_at(index).unwrap())
    }

    fn type_name(&self) -> &str {
        <dyn Struct>::type_name(*self)
    }
}

impl FieldAccess for &mut dyn TupleStruct {
    fn field_len(&self) -> usize {
        TupleStruct::field_len(*self)
    }

    fn field(&mut self, index: usize) -> &mut dyn Reflect {
        self.field_mut(index).unwrap()
    }

    fn name(&self, _: usize) -> Option<&str> {
        None
    }

    fn type_name(&self) -> &str {
        <dyn TupleStruct>::type_name(*self)
    }
}

impl FieldAccess for &mut dyn Tuple {
    fn field_len(&self) -> usize {
        Tuple::field_len(*self)
    }

    fn field(&mut self, index: usize) -> &mut dyn Reflect {
        self.field_mut(index).unwrap()
    }

    fn name(&self, _: usize) -> Option<&str> {
        None
    }

    fn type_name(&self) -> &str {
        <dyn Tuple>::type_name(*self)
    }
}

impl FieldAccess for &mut dyn Enum {
    fn field_len(&self) -> usize {
        Enum::field_len(*self)
    }

    fn field(&mut self, index: usize) -> &mut dyn Reflect {
        self.field_at_mut(index).unwrap()
    }

    fn name(&self, index: usize) -> Option<&str> {
        self.name_at(index)
    }

    fn type_name(&self) -> &str {
        <dyn Enum>::type_name(*self)
    }
}

pub fn composite_editor(
    ui: &mut Ui,
    mut repr: impl FieldAccess,
    world: &mut World,
    editors: &ReprEditors,
    states: &mut EditorStates,
    headless: bool,
) {
    let (fresh, state) = states.init(ui.id(), || EditorState::Composite);
    state.composite();

    let type_name = repr.type_name().to_string();

    let mut inner = |ui: &mut Ui| {
        ui.vertical(|ui| {
            for i in 0..repr.field_len() {
                ui.horizontal(|ui| {
                    ui.label(
                        repr.name(i)
                            .map(str::to_string)
                            .unwrap_or_else(|| format!(".{i}")),
                    );
                    let field = repr.field(i);
                    let editor = editors.get(field.type_name());
                    ui.push_id(i, |ui| {
                        if fresh {
                            states.remove(ui.id());
                        }
                        editor(ui, field, world, editors, states)
                    });
                });
            }
        })
    };

    if !headless {
        ui.collapsing(type_name, |ui| inner(ui));
    } else {
        inner(ui);
    }
}

pub fn list_editor(
    ui: &mut Ui,
    repr: &mut dyn List,
    world: &mut World,
    editors: &ReprEditors,
    states: &mut EditorStates,
) {
    let id = ui.id();
    let (fresh, _) = states.init(id, || EditorState::Composite);

    ui.collapsing(repr.type_name().to_string(), |ui| {
        ui.vertical(|ui| {
            let mut i = 0;
            loop {
                if i == repr.len() {
                    break;
                }

                ui.horizontal(|ui| {
                    let item = repr.get_mut(i).unwrap();
                    let editor = editors.get(item.type_name());
                    ui.label(format!("[{i}]"));
                    ui.push_id(i, |ui| {
                        if fresh {
                            states.remove(ui.id());
                        }
                        editor(ui, item, world, editors, states);
                    });
                    // TODO: Currently bevy's reflection capabilites are limiting when it comes to
                    // adding/removing from lists, so this is omitted for now.
                    // if ui.button("-").clicked() {
                    //     repr.remove(i);
                    //     i = i.wrapping_sub(1);
                    // }
                });

                i = i.wrapping_add(1);
            }

            states.ctors(id, |states, ctors| {
                let ctor = ctors.first();

                // TODO: Currently bevy's reflection capabilites are limiting when it comes to
                // adding/removing from lists, so this is omitted for now.
                // if ui.button("+").clicked() {
                //     match (|| {
                //         let item_name = match get_type_info(world, repr.type_name())? {
                //             TypeInfo::List(info) => info.item_type_name(),
                //             _ => todo!(),
                //             // TypeInfo::Dynamic(_) => ,
                //         };
                //         let item_info = get_type_info(world, item_name)?;
                //         default_value(item_info, world)
                //     })() {
                //         Some(item) => ctor.start(item),
                //         None => world
                //             .resource_mut::<Popups>()
                //             .add(Popup::new("failed to find reflection info")),
                //     }
                // }
            });
        })
    });
}

pub fn array_editor(
    ui: &mut Ui,
    repr: &mut dyn Array,
    world: &mut World,
    editors: &ReprEditors,
    states: &mut EditorStates,
) {
    let (fresh, state) = states.init(ui.id(), || EditorState::Composite);
    state.composite();

    ui.collapsing(repr.type_name().to_string(), |ui| {
        ui.vertical(|ui| {
            for i in 0..repr.len() {
                let item = repr.get_mut(i).unwrap();
                let editor = editors.get(item.type_name());
                ui.horizontal(|ui| {
                    ui.label(format!("[{i}]"));
                    ui.push_id(i, |ui| {
                        if fresh {
                            states.remove(ui.id());
                        }
                        editor(ui, item, world, editors, states);
                    });
                });
            }
        })
    });
}

pub fn map_editor(
    ui: &mut Ui,
    repr: &mut dyn Map,
    world: &mut World,
    editors: &ReprEditors,
    states: &mut EditorStates,
) {
    let id = ui.id();
    let (fresh, _) = states.init(id, || EditorState::Composite);

    ui.collapsing(repr.type_name().to_string(), |ui| {
        ui.vertical(|ui| {
            let repr_len = repr.len();
            let mut i = 0;
            loop {
                if i == repr_len {
                    break;
                }

                ui.horizontal(|ui| {
                    let (key, _) = repr.get_at(i).unwrap();
                    let key = key.clone_value();
                    ui.label(format!("[{i}] {key:?}: "));
                    let value = repr.get_mut(&*key).unwrap();
                    let value_editor = editors.get(value.type_name());
                    ui.push_id(repr_len + i, |ui| {
                        if fresh {
                            states.remove(ui.id());
                        }
                        value_editor(ui, &mut *value, world, editors, states);
                    });
                    // TODO: Currently bevy's reflection capabilites are limiting when it comes to
                    // adding/removing from lists, so this is omitted for now.
                    // if ui.button("-").clicked() {
                    //     repr.remove(i);
                    //     i = i.wrapping_sub(1);
                    // }
                });

                i = i.wrapping_add(1);
            }

            states.ctors(id, |states, ctors| {
                let ctor = ctors.first();

                // TODO: Currently bevy's reflection capabilites are limiting when it comes to
                // adding/removing from lists, so this is omitted for now.
                // if ui.button("+").clicked() {
                //     match (|| {
                //         let item_name = match get_type_info(world, repr.type_name())? {
                //             TypeInfo::List(info) => info.item_type_name(),
                //             _ => todo!(),
                //             // TypeInfo::Dynamic(_) => ,
                //         };
                //         let item_info = get_type_info(world, item_name)?;
                //         default_value(item_info, world)
                //     })() {
                //         Some(item) => ctor.start(item),
                //         None => world
                //             .resource_mut::<Popups>()
                //             .add(Popup::new("failed to find reflection info")),
                //     }
                // }
            });
        })
    });
}

pub fn enum_editor(
    ui: &mut Ui,
    repr: &mut dyn Enum,
    world: &mut World,
    editors: &ReprEditors,
    states: &mut EditorStates,
) {
    let id = ui.id();

    let Some(TypeInfo::Enum(info)) = get_type_info(world, repr.type_name()).cloned() else {
        ui.label("unable to reflect enum type");
        return;
    };

    ui.collapsing(repr.type_name().to_string(), |ui| {
        ui.vertical(|ui| {
            let button = variant_menu_button(ui, repr, &info, world, states, id);

            if button.response.lost_focus() {}

            let (fresh, state) = states.init(id, || EditorState::Composite);
            state.composite();

            states.ctors(id, |states, ctors| {
                if let Some(value) = ctors.first().poll(ui, world, editors, states) {
                    let variant = value.take::<VariantProxy>().unwrap();
                    let value = variant.into_enum(repr.type_name());
                    repr.apply(&value);
                }
            });

            match repr.variant_type() {
                VariantType::Unit => (),
                _ => {
                    ui.push_id(0, |ui| {
                        if fresh {
                            states.remove(ui.id());
                        }
                        composite_editor(ui, repr, world, editors, states, true)
                    });
                }
            }
        });
    });
}

fn variant_menu_button(
    ui: &mut Ui,
    repr: &mut dyn Enum,
    info: &EnumInfo,
    world: &World,
    states: &mut EditorStates,
    enum_id: egui::Id,
) -> InnerResponse<Option<()>> {
    ui.menu_button(repr.variant_name().to_string(), |ui| {
        ScrollArea::new([false, true]).show(ui, |ui| {
            for i in 0..info.variant_len() {
                let variant = info.variant_at(i).unwrap();
                if ui.button(variant.name()).clicked() {
                    if !ui.input(|i| i.modifiers.shift) {
                        ui.close_menu();
                    }

                    if let Some(value) = default_variant_value(variant, world) {
                        match variant {
                            VariantInfo::Unit(_) => {
                                let value = value.take::<VariantProxy>().unwrap();
                                repr.apply(&value.into_enum(repr.type_name()));
                            }
                            _ => states.ctors(enum_id, |_, ctors| {
                                ctors.first().start(value);
                            }),
                        }
                    } else {
                        // TODO: Failure
                    }
                }
            }
        });
    })
}

#[derive(Reflect)]
enum VariantKind {
    Struct(#[reflect(ignore)] DynamicStruct),
    Tuple(#[reflect(ignore)] DynamicTuple),
    Unit,
}

/// Holds a dynamic value and a variant tag to edit a variant
#[derive(Reflect)]
pub struct VariantProxy {
    variant: String,
    value: VariantKind,
}

impl VariantProxy {
    pub fn editor(
        ui: &mut Ui,
        repr: &mut dyn Reflect,
        world: &mut World,
        editors: &ReprEditors,
        states: &mut EditorStates,
    ) {
        let repr = repr.downcast_mut::<VariantProxy>().unwrap();
        match &mut repr.value {
            VariantKind::Struct(value) => {
                composite_editor(ui, value as &mut dyn Struct, world, editors, states, true);
            }
            VariantKind::Tuple(value) => {
                composite_editor(ui, value as &mut dyn Tuple, world, editors, states, true);
            }
            VariantKind::Unit => {}
        }
    }

    fn into_enum(self, name: &str) -> DynamicEnum {
        DynamicEnum::new(
            name,
            &self.variant,
            match self.value {
                VariantKind::Struct(value) => DynamicVariant::from(value),
                VariantKind::Tuple(value) => DynamicVariant::from(value),
                VariantKind::Unit => DynamicVariant::from(()),
            },
        )
    }
}

fn default_variant_value(variant: &VariantInfo, world: &World) -> Option<Box<dyn Reflect>> {
    match variant {
        VariantInfo::Struct(info) => {
            let mut value = DynamicStruct::default();
            for i in 0..info.field_len() {
                let field = info.field_at(i).unwrap();
                let info = get_type_info(world, field.type_name())?;
                value.insert_boxed(field.name(), default_value(info, world)?);
            }
            Some(Box::new(VariantProxy {
                variant: variant.name().to_string(),
                value: VariantKind::Struct(value),
            }))
        }
        VariantInfo::Tuple(info) => {
            let mut value = DynamicTuple::default();
            for i in 0..info.field_len() {
                let field = info.field_at(i).unwrap();
                let info = get_type_info(world, field.type_name())?;
                value.insert_boxed(default_value(info, world)?);
            }
            Some(Box::new(VariantProxy {
                variant: variant.name().to_string(),
                value: VariantKind::Tuple(value),
            }))
        }
        VariantInfo::Unit(_) => Some(Box::new(VariantProxy {
            variant: variant.name().to_string(),
            value: VariantKind::Unit,
        })),
    }
}

fn default_value(info: &TypeInfo, world: &World) -> Option<Box<dyn Reflect>> {
    match info {
        TypeInfo::Struct(info) => {
            let mut value = DynamicStruct::default();
            for i in 0..info.field_len() {
                let field = info.field_at(i).unwrap();
                let info = get_type_info(world, field.type_name())?;
                value.insert_boxed(field.name(), default_value(info, world)?);
            }
            Some(Box::new(value))
        }
        TypeInfo::TupleStruct(info) => {
            let mut value = DynamicTupleStruct::default();
            for i in 0..info.field_len() {
                let field = info.field_at(i).unwrap();
                let info = get_type_info(world, field.type_name())?;
                value.insert_boxed(default_value(info, world)?);
            }
            Some(Box::new(value))
        }
        TypeInfo::Tuple(info) => {
            let mut value = DynamicTuple::default();
            for i in 0..info.field_len() {
                let field = info.field_at(i).unwrap();
                let info = get_type_info(world, field.type_name())?;
                value.insert_boxed(default_value(info, world)?);
            }
            Some(Box::new(value))
        }
        TypeInfo::List(_) => {
            let value = DynamicList::default();
            Some(Box::new(value))
        }
        TypeInfo::Array(info) => {
            let item_info = get_type_info(world, info.item_type_name())?;
            let values = std::iter::repeat_with(|| default_value(item_info, world))
                .take(info.capacity())
                .collect::<Option<Vec<_>>>()?;
            let value = DynamicArray::new(values.into_boxed_slice());
            Some(Box::new(value))
        }
        TypeInfo::Map(_) => {
            let value = DynamicMap::default();
            Some(Box::new(value))
        }
        TypeInfo::Enum(info) => {
            let default_variant = info.variant_at(0)?;
            let default_value = default_variant_value(default_variant, world)?;
            let default_value: DynamicVariant = match default_value.reflect_ref() {
                bevy::reflect::ReflectRef::Struct(_) => {
                    (*default_value.downcast::<DynamicStruct>().unwrap()).into()
                }
                bevy::reflect::ReflectRef::Tuple(_) => {
                    (*default_value.downcast::<DynamicTuple>().unwrap()).into()
                }
                bevy::reflect::ReflectRef::Value(_) => ().into(),
                _ => unreachable!(),
            };
            let value = DynamicEnum::new(info.type_name(), default_variant.name(), default_value);
            Some(Box::new(value))
        }
        TypeInfo::Value(info) => match info.type_name() {
            "bool" => Some(Box::new(false)),
            "i8" => Some(Box::new(0i8)),
            "i16" => Some(Box::new(0i16)),
            "i32" => Some(Box::new(0i32)),
            "i64" => Some(Box::new(0i64)),
            "isize" => Some(Box::new(0isize)),
            "u8" => Some(Box::new(0u8)),
            "u16" => Some(Box::new(0u16)),
            "u32" => Some(Box::new(0u32)),
            "u64" => Some(Box::new(0u64)),
            "usize" => Some(Box::new(0usize)),
            "f32" => Some(Box::new(0.0f32)),
            "f64" => Some(Box::new(0.0f64)),
            "alloc::string::String" => Some(Box::new("".to_string())),
            _ => None,
        },
        TypeInfo::Dynamic(info) => match info.type_name() {
            "bevy::reflect::DynamicStruct" => Some(Box::<DynamicStruct>::default()),
            _ => None,
        },
    }
}

fn get_type_info<'w>(world: &'w World, name: &str) -> Option<&'w TypeInfo> {
    let registry = world.get_resource::<AppTypeRegistry>()?.read();
    Some(registry.get_with_name(name)?.type_info())
}

pub fn value_editor(ui: &mut Ui, repr: &mut dyn Reflect) {
    ui.vertical(|ui| {
        ui.label("No editor known for this value type. Consider adding an editor to ReprEditors");
        ui.label(format!("Debug representation: {repr:?}"));
    });
}

pub fn bool_editor(
    ui: &mut Ui,
    repr: &mut dyn Reflect,
    _: &mut World,
    _: &ReprEditors,
    _: &mut EditorStates,
) {
    let value = repr.downcast_mut::<bool>().unwrap();
    ui.checkbox(value, "");
}

pub fn num_editor<T: Copy + Reflect + FromStr + Display>(
    ui: &mut Ui,
    repr: &mut dyn Reflect,
    _: &mut World,
    _: &ReprEditors,
    states: &mut EditorStates,
) {
    let &value = repr.downcast_ref::<T>().unwrap();
    let text = states
        .get_or(ui.id(), || EditorState::TextEdit {
            temp_value: value.to_string(),
        })
        .text_edit();

    let edit = ui.text_edit_singleline(text);
    if edit.lost_focus() {
        let value = text.parse::<T>().unwrap_or(value);
        states.remove(ui.id());
        repr.apply(&value);
    }
    if !edit.has_focus() {
        states.remove(ui.id());
    }
}

pub fn string_editor(
    ui: &mut Ui,
    repr: &mut dyn Reflect,
    _: &mut World,
    _: &ReprEditors,
    states: &mut EditorStates,
) {
    let value = repr.downcast_ref::<String>().unwrap();
    let text = states
        .get_or(ui.id(), || EditorState::TextEdit {
            temp_value: value.into(),
        })
        .text_edit();
    let edit = ui.text_edit_singleline(text);
    if edit.lost_focus() {
        repr.apply(text);
        states.remove(ui.id());
    }
    if !edit.has_focus() {
        states.remove(ui.id());
    }
}
