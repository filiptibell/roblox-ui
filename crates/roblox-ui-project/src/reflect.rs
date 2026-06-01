/*!
    Thin read-only helpers over the bundled reflection database. We never build
    or mutate reflection data — only query it.
*/

use rbx_dom_weak::{types::Variant, UstrMap};
use rbx_reflection::{ClassTag, DataType, ReflectionDatabase};

/**
    The process-wide reflection database (bundled with `rbx_reflection_database`).
*/
pub fn database() -> &'static ReflectionDatabase<'static> {
    rbx_reflection_database::get().expect("failed to load reflection database")
}

/**
    Whether `class` is, or descends from, `target` (the Roblox `IsA` relation).
*/
pub fn class_is_a(db: &ReflectionDatabase<'_>, class: &str, target: &str) -> bool {
    if class.eq_ignore_ascii_case(target) {
        return true;
    }
    let mut current = db.classes.get(class);
    while let Some(descriptor) = current {
        if descriptor.name.eq_ignore_ascii_case(target) {
            return true;
        }
        current = descriptor
            .superclass
            .as_ref()
            .and_then(|sc| db.classes.get(sc.as_ref()));
    }
    false
}

/**
    Whether `name` is a class tagged as a `Service` (e.g. `ReplicatedStorage`,
    `Workspace`). Used to resolve unqualified DataModel children, like Rojo.
*/
pub fn is_service(db: &ReflectionDatabase<'_>, name: &str) -> bool {
    db.classes
        .get(name)
        .map(|c| c.tags.contains(&ClassTag::Service))
        .unwrap_or(false)
}

/**
    A property's declared [`DataType`], walking the superclass chain.
*/
pub fn property_data_type<'a>(
    db: &'a ReflectionDatabase<'_>,
    class: &str,
    prop: &str,
) -> Option<DataType<'a>> {
    let mut current = db.classes.get(class);
    while let Some(descriptor) = current {
        if let Some(property) = descriptor.properties.get(prop) {
            return Some(property.data_type.clone());
        }
        current = descriptor
            .superclass
            .as_ref()
            .and_then(|sc| db.classes.get(sc.as_ref()));
    }
    None
}

/**
    The name of the enum item with the given value (reverse of the usual
    name → value map), e.g. `(Material, 256) -> "Plastic"`.
*/
pub fn enum_item_name(db: &ReflectionDatabase<'_>, enum_name: &str, value: u32) -> Option<String> {
    let descriptor = db.enums.get(enum_name)?;
    descriptor
        .items
        .iter()
        .find(|(_, v)| **v == value)
        .map(|(name, _)| name.to_string())
}

/**
    The class default for a property, walking the superclass chain.
*/
pub fn default_value<'a>(
    db: &'a ReflectionDatabase,
    class: &str,
    prop: &str,
) -> Option<&'a Variant> {
    let mut current = db.classes.get(class);
    while let Some(descriptor) = current {
        if let Some(value) = descriptor.default_properties.get(prop) {
            return Some(value);
        }
        current = descriptor
            .superclass
            .as_ref()
            .and_then(|sc| db.classes.get(sc.as_ref()));
    }
    None
}

/**
    Drop any property whose value equals the class default, so we store and
    diff only genuine overrides.
*/
pub fn default_diff(db: &ReflectionDatabase<'_>, class: &str, props: &mut UstrMap<Variant>) {
    props.retain(|name, value| match default_value(db, class, name) {
        Some(default) => default != value,
        None => true,
    });
}
