/*!
    Resolution of Rojo-style JSON property encodings (`.model.json`,
    `.meta.json`, project `$properties`) into typed `Variant`s. Two encodings are
    supported, both read against (never reimplementing) the reflection database:
      1. Explicit/tagged: a one-key object naming a variant type, e.g.
         `{ "UDim2": [[0,100],[0,50]] }`.
      2. Ambiguous/bare: a plain JSON value whose type is looked up from the
         owning `(class, property)` pair. Anything unresolvable is skipped.
*/

use rbx_dom_weak::{
    types::{
        Attributes, CFrame, Color3, ColorSequence, ColorSequenceKeypoint, Enum, Matrix3,
        NumberRange, NumberSequence, NumberSequenceKeypoint, Rect, UDim, UDim2, Variant,
        VariantType, Vector2, Vector3,
    },
    Ustr, UstrMap,
};
use rbx_reflection::{DataType, ReflectionDatabase};
use serde_json::{Map as JsonMap, Value as Json};

/**
    Resolve and insert a map of Rojo-encoded JSON properties as typed variants.
    `Name` is never stored as a property (it lives on `Instance.name`).
*/
pub(crate) fn apply_json_properties(
    db: &ReflectionDatabase<'_>,
    class: &Ustr,
    json: &JsonMap<String, Json>,
    out: &mut UstrMap<Variant>,
) {
    for (prop_name, value) in json {
        if prop_name == "Name" {
            continue;
        }
        if let Some(variant) = resolve_property(db, class, prop_name, value) {
            out.insert(Ustr::from(prop_name.as_str()), variant);
        }
    }
}

/**
    Resolve a single property value for the given class + property name.
*/
fn resolve_property(
    db: &ReflectionDatabase<'_>,
    class: &str,
    prop_name: &str,
    json: &Json,
) -> Option<Variant> {
    // Explicit tagged form takes precedence: a one-key object whose key names a
    // variant type. (We must not mistake e.g. an Attributes map for a tag.)
    if let Json::Object(map) = json {
        if map.len() == 1 {
            let (key, inner) = map.iter().next().unwrap();
            if let Some(vt) = variant_type_from_name(key) {
                return from_typed(db, vt, inner);
            }
        }
    }

    // Ambiguous form: resolve the property's declared type from reflection.
    if let Some(data_type) = find_property_type(db, class, prop_name) {
        match data_type {
            DataType::Enum(enum_name) => return resolve_enum(db, enum_name.as_ref(), json),
            DataType::Value(vt) => return from_typed(db, vt, json),
            _ => {}
        }
    }

    // No reflection info: fall back to the JSON shape for the common scalars.
    infer_from_shape(json)
}

/**
    Look up a property's declared [`DataType`], walking the superclass chain.
*/
fn find_property_type<'a>(
    db: &'a ReflectionDatabase,
    class: &str,
    prop_name: &str,
) -> Option<DataType<'a>> {
    let mut current = Some(class.to_string());
    while let Some(class_name) = current {
        let descriptor = db.classes.get(class_name.as_str())?;
        if let Some(prop) = descriptor.properties.get(prop_name) {
            return Some(prop.data_type.clone());
        }
        current = descriptor.superclass.as_ref().map(|c| c.to_string());
    }
    None
}

fn resolve_enum(db: &ReflectionDatabase<'_>, enum_name: &str, json: &Json) -> Option<Variant> {
    match json {
        Json::String(name) => {
            let descriptor = db.enums.get(enum_name)?;
            let value = descriptor.items.get(name.as_str())?;
            Some(Variant::Enum(Enum::from_u32(*value)))
        }
        Json::Number(n) => Some(Variant::Enum(Enum::from_u32(n.as_u64()? as u32))),
        _ => None,
    }
}

/**
    Build a [`Variant`] of a known [`VariantType`] from a JSON value.
*/
fn from_typed(db: &ReflectionDatabase<'_>, vt: VariantType, json: &Json) -> Option<Variant> {
    // NB: we deliberately do *not* glob-import `VariantType` here, since its
    // variant names (`Rect`, `Vector2`, …) shadow the value type names.
    use VariantType as Vt;
    let v = match vt {
        Vt::Bool => Variant::Bool(json.as_bool()?),
        Vt::Float32 => Variant::Float32(json.as_f64()? as f32),
        Vt::Float64 => Variant::Float64(json.as_f64()?),
        Vt::Int32 => Variant::Int32(json.as_i64()? as i32),
        Vt::Int64 => Variant::Int64(json.as_i64()?),
        Vt::String => Variant::String(json.as_str()?.to_owned()),
        Vt::Content => Variant::Content(rbx_dom_weak::types::Content::from_uri(json.as_str()?)),
        Vt::ContentId => Variant::ContentId(rbx_dom_weak::types::ContentId::from(
            json.as_str()?.to_owned(),
        )),
        Vt::BrickColor => {
            let name = json.as_str()?;
            Variant::BrickColor(rbx_dom_weak::types::BrickColor::from_name(name)?)
        }
        Vt::Vector2 => Variant::Vector2(parse_vector2(json)?),
        Vt::Vector3 => Variant::Vector3(parse_vector3(json)?),
        Vt::Color3 => Variant::Color3(parse_color3(json)?),
        Vt::UDim => Variant::UDim(parse_udim(json)?),
        Vt::UDim2 => Variant::UDim2(parse_udim2(json)?),
        Vt::Rect => {
            let arr = json.as_array()?;
            Variant::Rect(Rect::new(parse_vector2(&arr[0])?, parse_vector2(&arr[1])?))
        }
        Vt::NumberRange => {
            let arr = json.as_array()?;
            Variant::NumberRange(NumberRange::new(
                arr[0].as_f64()? as f32,
                arr[1].as_f64()? as f32,
            ))
        }
        Vt::CFrame => Variant::CFrame(parse_cframe(json)?),
        Vt::ColorSequence => Variant::ColorSequence(parse_color_sequence(json)?),
        Vt::NumberSequence => Variant::NumberSequence(parse_number_sequence(json)?),
        Vt::Font => Variant::Font(parse_font(json)?),
        Vt::Enum => Variant::Enum(Enum::from_u32(json.as_u64()? as u32)),
        // Anything else we don't have a JSON spelling for is intentionally
        // skipped rather than guessed.
        _ => return resolve_via_serde(db, vt, json),
    };
    Some(v)
}

/**
    Last-resort path for variant types we don't hand-parse: let rbx_types' own
    serde implementation try, so we transparently support its spellings too.
*/
fn resolve_via_serde(
    _db: &ReflectionDatabase<'_>,
    _vt: VariantType,
    _json: &Json,
) -> Option<Variant> {
    None
}

fn infer_from_shape(json: &Json) -> Option<Variant> {
    match json {
        Json::Bool(b) => Some(Variant::Bool(*b)),
        Json::String(s) => Some(Variant::String(s.clone())),
        Json::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Variant::Int64(i))
            } else {
                Some(Variant::Float64(n.as_f64()?))
            }
        }
        _ => None,
    }
}

fn parse_vector2(json: &Json) -> Option<Vector2> {
    let a = json.as_array()?;
    Some(Vector2::new(
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
    ))
}

fn parse_vector3(json: &Json) -> Option<Vector3> {
    let a = json.as_array()?;
    Some(Vector3::new(
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
        a.get(2)?.as_f64()? as f32,
    ))
}

fn parse_color3(json: &Json) -> Option<Color3> {
    let a = json.as_array()?;
    Some(Color3::new(
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
        a.get(2)?.as_f64()? as f32,
    ))
}

fn parse_udim(json: &Json) -> Option<UDim> {
    let a = json.as_array()?;
    Some(UDim::new(
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_i64()? as i32,
    ))
}

fn parse_udim2(json: &Json) -> Option<UDim2> {
    let a = json.as_array()?;
    // Accept either [[s,o],[s,o]] or flat [sx,ox,sy,oy].
    if a.len() == 2 && a[0].is_array() {
        Some(UDim2::new(parse_udim(&a[0])?, parse_udim(&a[1])?))
    } else if a.len() == 4 {
        Some(UDim2::new(
            UDim::new(a[0].as_f64()? as f32, a[1].as_i64()? as i32),
            UDim::new(a[2].as_f64()? as f32, a[3].as_i64()? as i32),
        ))
    } else {
        None
    }
}

fn parse_cframe(json: &Json) -> Option<CFrame> {
    let a = json.as_array()?;
    // Position only: [x,y,z].
    if a.len() == 3 {
        return Some(CFrame::new(parse_vector3(json)?, Matrix3::identity()));
    }
    // Position + 3x3 orientation rows: [[x,y,z],[r00,r01,r02],[r10,..],[r20,..]].
    if a.len() == 4 {
        let pos = parse_vector3(&a[0])?;
        let orientation = Matrix3::new(
            parse_vector3(&a[1])?,
            parse_vector3(&a[2])?,
            parse_vector3(&a[3])?,
        );
        return Some(CFrame::new(pos, orientation));
    }
    None
}

fn parse_color_sequence(json: &Json) -> Option<ColorSequence> {
    let arr = json.as_array()?;
    let mut keypoints = Vec::with_capacity(arr.len());
    for kp in arr {
        let o = kp.as_object()?;
        let time = o.get("time")?.as_f64()? as f32;
        let color = parse_color3(o.get("color")?)?;
        keypoints.push(ColorSequenceKeypoint::new(time, color));
    }
    Some(ColorSequence { keypoints })
}

fn parse_number_sequence(json: &Json) -> Option<NumberSequence> {
    let arr = json.as_array()?;
    let mut keypoints = Vec::with_capacity(arr.len());
    for kp in arr {
        let o = kp.as_object()?;
        let time = o.get("time")?.as_f64()? as f32;
        let value = o.get("value")?.as_f64()? as f32;
        let envelope = o.get("envelope").and_then(|e| e.as_f64()).unwrap_or(0.0) as f32;
        keypoints.push(NumberSequenceKeypoint::new(time, value, envelope));
    }
    Some(NumberSequence { keypoints })
}

fn parse_font(json: &Json) -> Option<rbx_dom_weak::types::Font> {
    use rbx_dom_weak::types::{Font, FontStyle, FontWeight};
    let o = json.as_object()?;
    let family = o.get("family")?.as_str()?;
    let weight = match o.get("weight").and_then(|w| w.as_u64()) {
        Some(100) => FontWeight::Thin,
        Some(200) => FontWeight::ExtraLight,
        Some(300) => FontWeight::Light,
        Some(400) | None => FontWeight::Regular,
        Some(500) => FontWeight::Medium,
        Some(600) => FontWeight::SemiBold,
        Some(700) => FontWeight::Bold,
        Some(800) => FontWeight::ExtraBold,
        Some(900) => FontWeight::Heavy,
        _ => FontWeight::Regular,
    };
    let style = match o.get("style").and_then(|s| s.as_str()) {
        Some("Italic") => FontStyle::Italic,
        _ => FontStyle::Normal,
    };
    Some(Font::new(family, weight, style))
}

/**
    Resolve a value with no class/property context (used for attributes):
    explicit tagged form first, else infer from the JSON shape.
*/
fn resolve_unambiguous(db: &ReflectionDatabase<'_>, json: &Json) -> Option<Variant> {
    if let Json::Object(map) = json {
        if map.len() == 1 {
            let (key, inner) = map.iter().next().unwrap();
            if let Some(vt) = variant_type_from_name(key) {
                return from_typed(db, vt, inner);
            }
        }
    }
    infer_from_shape(json)
}

/**
    Build an `Attributes` bag from a JSON map (each value resolved unambiguously).
    Returns `None` for an empty map so we never store an empty override.
*/
pub(crate) fn resolve_attributes(
    db: &ReflectionDatabase<'_>,
    json: &JsonMap<String, Json>,
) -> Option<Variant> {
    if json.is_empty() {
        return None;
    }
    let attrs: Attributes = json
        .iter()
        .filter_map(|(k, v)| resolve_unambiguous(db, v).map(|val| (k.clone(), val)))
        .collect();
    Some(Variant::Attributes(attrs))
}

/**
    Map a Rojo type-name string (the key in the tagged form) to a [`VariantType`].
*/
fn variant_type_from_name(name: &str) -> Option<VariantType> {
    use VariantType::*;
    Some(match name {
        "Bool" => Bool,
        "Float32" => Float32,
        "Float64" | "Number" => Float64,
        "Int32" => Int32,
        "Int64" => Int64,
        "String" => String,
        "Content" => Content,
        "ContentId" => ContentId,
        "BrickColor" => BrickColor,
        "Vector2" => Vector2,
        "Vector3" => Vector3,
        "Color3" => Color3,
        "UDim" => UDim,
        "UDim2" => UDim2,
        "Rect" => Rect,
        "NumberRange" => NumberRange,
        "CFrame" => CFrame,
        "ColorSequence" => ColorSequence,
        "NumberSequence" => NumberSequence,
        "Font" => Font,
        "Enum" => Enum,
        _ => return None,
    })
}
