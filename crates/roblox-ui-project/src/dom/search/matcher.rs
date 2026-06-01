/*!
    Matcher for the Studio explorer search grammar.

    Evaluates a parsed [`Query`] against a single instance: name / `is:` / `tag:`
    tests, property comparisons (with sub-fields, quoted composite units,
    enum-by-name, alias resolution, and class-default fallback so comparisons see
    the *effective* value), and ancestry-path matching against the instance's
    ancestor chain.
*/

use rbx_dom_weak::{
    types::{Ref, Variant},
    Ustr,
};
use rbx_reflection::{DataType, PropertyKind, ReflectionDatabase};

use crate::reflect::{class_is_a, database, default_value, enum_item_name};

use super::super::Dom;
use super::ast::{Op, Query, Seg};

/**
    Components closer than this are considered equal (and numbers, for `=`/`~=`).
*/
const EPSILON: f64 = 1e-6;

pub(crate) fn matches(dom: &Dom, id: Ref, query: &Query) -> bool {
    match query {
        Query::And(parts) => parts.iter().all(|part| matches(dom, id, part)),
        Query::Or(parts) => parts.iter().any(|part| matches(dom, id, part)),
        Query::Name(needle) => name_matches(dom, id, needle),
        Query::Is(class) => dom
            .get_instance(id)
            .is_some_and(|inst| class_is_a(database(), inst.class.as_str(), class)),
        Query::Tag(tag) => tag_matches(dom, id, tag),
        Query::Property { path, op, value } => property_matches(dom, id, path, *op, value),
        Query::Ancestry(segs) => ancestry_matches(dom, id, segs),
    }
}

fn name_matches(dom: &Dom, id: Ref, needle: &str) -> bool {
    dom.get_instance(id)
        .is_some_and(|inst| inst.name.to_lowercase().contains(&needle.to_lowercase()))
}

fn tag_matches(dom: &Dom, id: Ref, tag: &str) -> bool {
    match dom
        .get_properties(id)
        .and_then(|p| p.get(&Ustr::from("Tags")))
    {
        Some(Variant::Tags(tags)) => tags.iter().any(|t| t.eq_ignore_ascii_case(tag)),
        _ => false,
    }
}

// ---- property comparison --------------------------------------------------

/**
    A property value resolved to a comparable form.
*/
enum Val {
    Str(String),
    Num(f64),
    Bool(bool),
    /// A composite's components (e.g. Vector3 / Color3), for quoted-unit compares.
    Components(Vec<f64>),
}

fn property_matches(dom: &Dom, id: Ref, path: &[String], op: Op, query: &str) -> bool {
    let Some(prop) = path.first() else {
        return false;
    };
    let sub = path.get(1).map(String::as_str);
    let Some(inst) = dom.get_instance(id) else {
        return false;
    };

    // `ClassName` / `Name` are modelled on the instance, not as properties.
    if prop.eq_ignore_ascii_case("ClassName") {
        return compare(&Val::Str(inst.class.to_string()), op, query);
    }
    if prop.eq_ignore_ascii_case("Name") {
        return compare(&Val::Str(inst.name.clone()), op, query);
    }

    let db = database();
    let class = inst.class.as_str();

    // `Position` / `Orientation` read as their own canonical properties in the
    // reflection database, but rbx-dom stores the data on `CFrame`; derive the
    // positional components so `Position.X` etc. work.
    if prop.eq_ignore_ascii_case("Position") {
        return position_matches(dom, id, sub, op, query);
    }

    // Resolve the canonical, alias-followed property name (case- and
    // space-insensitive), then read the effective value (explicit override, or
    // the class default - our property bags are default-diffed).
    let Some(canonical) = canonical_property(db, class, prop) else {
        return false;
    };
    let variant = inst
        .properties
        .get(&Ustr::from(canonical.as_str()))
        .cloned()
        .or_else(|| default_value(db, class, &canonical).cloned());
    let Some(variant) = variant else {
        return false;
    };

    match variant_to_val(db, class, &canonical, &variant, sub) {
        Some(actual) => compare(&actual, op, query),
        None => false,
    }
}

fn position_matches(dom: &Dom, id: Ref, sub: Option<&str>, op: Op, query: &str) -> bool {
    let db = database();
    let inst = match dom.get_instance(id) {
        Some(inst) => inst,
        None => return false,
    };
    let cframe = inst
        .properties
        .get(&Ustr::from("CFrame"))
        .cloned()
        .or_else(|| default_value(db, inst.class.as_str(), "CFrame").cloned());
    let Some(Variant::CFrame(cframe)) = cframe else {
        return false;
    };
    let p = cframe.position;
    let actual = match sub {
        Some(axis) => axis_component(&[("x", p.x), ("y", p.y), ("z", p.z)], axis),
        None => Some(Val::Components(vec![p.x as f64, p.y as f64, p.z as f64])),
    };
    actual.is_some_and(|val| compare(&val, op, query))
}

/**
    Resolve a (possibly mis-cased / spaced) property name to the canonical name
    under which its value is stored, following aliases (e.g. `Color` ->
    `Color3uint8`).
*/
fn canonical_property(db: &ReflectionDatabase, class: &str, prop: &str) -> Option<String> {
    let wanted = normalize(prop);
    let mut current = db.classes.get(class);
    while let Some(descriptor) = current {
        for (name, property) in &descriptor.properties {
            if normalize(name) == wanted {
                return Some(match &property.kind {
                    PropertyKind::Alias { alias_for } => alias_for.to_string(),
                    _ => name.to_string(),
                });
            }
        }
        current = descriptor
            .superclass
            .as_ref()
            .and_then(|sc| db.classes.get(sc.as_ref()));
    }
    None
}

fn variant_to_val(
    db: &ReflectionDatabase,
    class: &str,
    prop: &str,
    variant: &Variant,
    sub: Option<&str>,
) -> Option<Val> {
    match variant {
        Variant::Bool(b) => Some(Val::Bool(*b)),
        Variant::Float32(f) => Some(Val::Num(f64::from(*f))),
        Variant::Float64(f) => Some(Val::Num(*f)),
        Variant::Int32(i) => Some(Val::Num(f64::from(*i))),
        Variant::Int64(i) => Some(Val::Num(*i as f64)),
        Variant::String(s) => Some(Val::Str(s.clone())),
        Variant::ContentId(c) => Some(Val::Str(c.as_str().to_string())),
        Variant::Content(c) => c.as_uri().map(|uri| Val::Str(uri.to_string())),
        Variant::Enum(e) => match crate::reflect::property_data_type(db, class, prop) {
            Some(DataType::Enum(name)) => {
                enum_item_name(db, name.as_ref(), e.to_u32()).map(Val::Str)
            }
            _ => Some(Val::Num(f64::from(e.to_u32()))),
        },
        Variant::Vector3(v) => match sub {
            Some(axis) => axis_component(&[("x", v.x), ("y", v.y), ("z", v.z)], axis),
            None => Some(Val::Components(vec![v.x as f64, v.y as f64, v.z as f64])),
        },
        Variant::Vector2(v) => match sub {
            Some(axis) => axis_component(&[("x", v.x), ("y", v.y)], axis),
            None => Some(Val::Components(vec![v.x as f64, v.y as f64])),
        },
        // Studio presents Color3 channels on a 0-255 scale.
        Variant::Color3(c) => color_val(c.r * 255.0, c.g * 255.0, c.b * 255.0, sub),
        Variant::Color3uint8(c) => color_val(f32::from(c.r), f32::from(c.g), f32::from(c.b), sub),
        _ => None,
    }
}

fn color_val(r: f32, g: f32, b: f32, sub: Option<&str>) -> Option<Val> {
    match sub {
        Some(channel) => axis_component(&[("r", r), ("g", g), ("b", b)], channel),
        None => Some(Val::Components(vec![r as f64, g as f64, b as f64])),
    }
}

fn axis_component(axes: &[(&str, f32)], wanted: &str) -> Option<Val> {
    axes.iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
        .map(|(_, value)| Val::Num(f64::from(*value)))
}

fn compare(actual: &Val, op: Op, query: &str) -> bool {
    match actual {
        Val::Str(a) => {
            // Partial, case-insensitive, space-insensitive (Roblox: `== plas`
            // matches `Plastic`). Ordering operators are meaningless on strings.
            let (a, q) = (normalize(a), normalize(query));
            match op {
                Op::Eq => a.contains(&q),
                Op::Ne => !a.contains(&q),
                _ => false,
            }
        }
        Val::Bool(a) => match parse_bool(query) {
            Some(q) => match op {
                Op::Eq => *a == q,
                Op::Ne => *a != q,
                _ => false,
            },
            None => false,
        },
        Val::Num(a) => match query.trim().parse::<f64>() {
            Ok(q) => compare_num(*a, op, q),
            Err(_) => false,
        },
        Val::Components(a) => match parse_components(query) {
            Some(q) if q.len() == a.len() => {
                let component_eq = || a.iter().zip(&q).all(|(x, y)| (x - y).abs() < EPSILON);
                match op {
                    Op::Eq => component_eq(),
                    Op::Ne => !component_eq(),
                    _ => a.iter().zip(&q).all(|(x, y)| compare_num(*x, op, *y)),
                }
            }
            _ => false,
        },
    }
}

fn compare_num(a: f64, op: Op, q: f64) -> bool {
    match op {
        Op::Eq => (a - q).abs() < EPSILON,
        Op::Ne => (a - q).abs() >= EPSILON,
        Op::Gt => a > q,
        Op::Ge => a >= q,
        Op::Lt => a < q,
        Op::Le => a <= q,
    }
}

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_components(s: &str) -> Option<Vec<f64>> {
    s.split(',')
        .map(|part| part.trim().parse::<f64>().ok())
        .collect()
}

// ---- ancestry -------------------------------------------------------------

fn ancestry_matches(dom: &Dom, id: Ref, segs: &[Seg]) -> bool {
    // The chain of names from the instance up to the root.
    let mut chain = Vec::new();
    let mut current = id;
    while let Some(inst) = dom.get_instance(current) {
        chain.push(inst.name.clone());
        let parent = inst.parent();
        if parent.is_none() {
            break;
        }
        current = parent;
    }

    // Match the pattern right-anchored at the instance (segments reversed so the
    // target lines up with the instance itself). The left end is unanchored:
    // ancestors above the matched run are ignored.
    let reversed: Vec<&Seg> = segs.iter().rev().collect();
    match_segs(&reversed, &chain)
}

fn match_segs(segs: &[&Seg], chain: &[String]) -> bool {
    match segs.split_first() {
        None => true,
        Some((seg, rest)) => match seg {
            Seg::Name(name) => {
                !chain.is_empty()
                    && chain[0].eq_ignore_ascii_case(name)
                    && match_segs(rest, &chain[1..])
            }
            Seg::AnyOne => !chain.is_empty() && match_segs(rest, &chain[1..]),
            Seg::AnyDepth => (1..=chain.len()).any(|i| match_segs(rest, &chain[i..])),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_compare_is_partial_and_insensitive() {
        let plastic = Val::Str("SmoothPlastic".into());
        assert!(compare(&plastic, Op::Eq, "plas"));
        assert!(!compare(&plastic, Op::Eq, "metal"));
        assert!(compare(&plastic, Op::Ne, "metal"));
    }

    #[test]
    fn bool_compare_accepts_only_true_false() {
        assert!(compare(&Val::Bool(true), Op::Eq, "true"));
        assert!(compare(&Val::Bool(true), Op::Ne, "false"));
        assert!(!compare(&Val::Bool(true), Op::Eq, "1"));
    }

    #[test]
    fn number_compare_operators() {
        assert!(compare(&Val::Num(0.5), Op::Eq, "0.5"));
        assert!(compare(&Val::Num(0.5), Op::Gt, "0.4"));
        assert!(compare(&Val::Num(1.0), Op::Ge, "1"));
        assert!(!compare(&Val::Num(0.5), Op::Lt, "0.4"));
    }

    #[test]
    fn component_compare_is_elementwise() {
        let size = Val::Components(vec![20.0, 5.0, 20.0]);
        assert!(compare(&size, Op::Gt, "10, 1, 10"));
        assert!(!compare(&size, Op::Gt, "10, 10, 10")); // 5 is not > 10
        assert!(compare(&size, Op::Eq, "20, 5, 20"));
    }
}
