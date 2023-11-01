use std::collections::{BTreeMap, VecDeque};

use anyhow::{bail, Context, Result};
use quick_xml::events::{BytesStart, Event as XmlEvent};
use quick_xml::Reader as XmlReader;

use super::value::*;

// HACK: Some values dont have proper types in reflection
// metadata, we try to coerce them manually to fix that
const VALUE_COERCIONS: &[(&str, ValueKind)] = &[
    ("ExplorerOrder", ValueKind::Integer),
    ("ExplorerImageIndex", ValueKind::Integer),
    ("ServiceVisibility", ValueKind::Integer),
    ("Deprecated", ValueKind::Bool),
    ("Insertable", ValueKind::Bool),
    ("Browsable", ValueKind::Bool),
];

#[derive(Debug, Clone)]
pub enum Node {
    Root {
        children: Vec<Node>,
    },
    Item {
        name: String,
        children: Vec<Node>,
    },
    Properties {
        children: Vec<Node>,
    },
    Value {
        kind: ValueKind,
        name: String,
        value: Value,
    },
}

impl Node {
    fn new_props() -> Self {
        Self::Properties {
            children: Vec::new(),
        }
    }

    fn new_root() -> Self {
        Self::Root {
            children: Vec::new(),
        }
    }

    fn new_item(e: &BytesStart<'_>) -> Result<Self> {
        let name = e
            .try_get_attribute("class")
            .context("failed to get xml attribute")?
            .context("missing class attribute for item node")?
            .unescape_value()?
            .trim_start_matches("ReflectionMetadata")
            .to_string();
        Ok(Self::Item {
            name,
            children: Vec::new(),
        })
    }

    fn new_value(e: &BytesStart<'_>) -> Result<Self> {
        let kind_str = String::from_utf8(e.name().0.to_vec())
            .context("failed to parse event name - must be valid utf-8")?;
        let kind = kind_str
            .parse::<ValueKind>()
            .context("failed to parse event kind")?;

        let name = e
            .try_get_attribute("name")
            .context("failed to get xml attribute")?
            .context("missing name attribute for value node")?
            .unescape_value()?
            .into_owned();

        Ok(Self::Value {
            kind,
            name,
            value: Value::None,
        })
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Self::Root { children, .. }
            | Self::Item { children, .. }
            | Self::Properties { children, .. } => Some(children),
            _ => None,
        }
    }

    pub fn as_value(&self) -> Option<&Value> {
        match self {
            Self::Value { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Item {
                name: class_name, ..
            } => Some(class_name),
            Self::Value { name, .. } => Some(name),
            _ => None,
        }
    }

    pub fn children(&self) -> &[Node] {
        match self {
            Self::Root { children, .. }
            | Self::Item { children, .. }
            | Self::Properties { children, .. } => children,
            _ => &[],
        }
    }

    pub fn split_properties(&self) -> Option<(&Node, Vec<&Node>)> {
        let (props_idx, props) = self
            .children()
            .iter()
            .enumerate()
            .find(|(_, child)| matches!(child, Node::Properties { .. }))?;
        let rest = self
            .children()
            .iter()
            .enumerate()
            .filter_map(|(idx, node)| if idx != props_idx { Some(node) } else { None })
            .collect::<Vec<_>>();
        Some((props, rest))
    }

    pub fn find_child(&self, predicate: impl Fn(&Node) -> bool) -> Option<&Node> {
        self.children().iter().find(|&child| predicate(child))
    }

    pub fn extract_name_node_string(&self) -> Result<String> {
        Ok(self
            .find_child(|child| matches!(child.name(), Some("Name")))
            .context("enum node 'Name' child did not exist")?
            .as_value()
            .context("enum node 'Name' child was not a Value node")?
            .coerce(ValueKind::String)
            .context("enum node 'Name' child was not a string Value node")?
            .as_string()
            .unwrap()
            .to_string())
    }

    pub fn extract_non_name_values(&self) -> Result<BTreeMap<String, Value>> {
        if let Self::Properties { children } = self {
            let mut values = BTreeMap::new();

            for child_node in children {
                let prop_name = match child_node.name() {
                    Some(n) => n,
                    None => continue,
                };
                if prop_name.eq_ignore_ascii_case("name") {
                    continue;
                }
                let prop_value = match child_node.as_value() {
                    Some(v) => v,
                    None => continue,
                };
                values.insert(prop_name.to_string(), prop_value.clone());
            }

            Ok(values)
        } else {
            bail!("cannot extract values from a node that is not Properties")
        }
    }
}

pub fn parse_reflection_tree(reflection_bytes: &[u8]) -> Result<Node> {
    let mut reader = XmlReader::from_reader(reflection_bytes);
    reader.trim_markup_names_in_closing_tags(true);
    reader.trim_text(true);

    let mut xml_events = Vec::new();
    loop {
        match reader.read_event().with_context(|| {
            format!("error reading xml at position {}", reader.buffer_position())
        })? {
            XmlEvent::Eof => break,
            e => xml_events.push(e),
        }
    }

    let mut node_stack = VecDeque::new();
    node_stack.push_back(Node::new_root());

    for event in &xml_events {
        match event {
            XmlEvent::Start(e) => {
                let mut created = match e.name().0 {
                    b"roblox" => continue,
                    b"Properties" => Node::new_props(),
                    b"Item" => Node::new_item(e)?,
                    _ => Node::new_value(e)?,
                };
                if created.children_mut().is_some() {
                    node_stack.push_front(created);
                } else {
                    let current_node = node_stack.front_mut().unwrap();
                    current_node
                        .children_mut()
                        .expect("event stack was not created correctly")
                        .push(created);
                }
            }
            XmlEvent::End(e) => match e.name().0 {
                b"roblox" => continue,
                b"Properties" | b"Item" => {
                    let current_node = node_stack.pop_front().unwrap();
                    let parent_node = node_stack.front_mut().unwrap();
                    parent_node
                        .children_mut()
                        .expect("event stack was not created correctly")
                        .push(current_node);
                }
                _ => continue,
            },
            XmlEvent::Text(e) => {
                let current_node = node_stack.front_mut().unwrap();
                let current_children = current_node
                    .children_mut()
                    .expect("event stack was not created correctly");
                let last_inserted_child = current_children
                    .iter_mut()
                    .rev()
                    .find(|child| matches!(child, Node::Value { .. }));
                if let Some(Node::Value {
                    kind,
                    name,
                    ref mut value,
                }) = last_inserted_child
                {
                    let value_str: &str = &e.unescape()?;
                    let value_new = Value::parse(*kind, name, value_str).with_context(|| {
                        format!("failed to parse value '{value_str}' as {kind} for prop '{name}'")
                    })?;

                    let coerced = match VALUE_COERCIONS
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(name))
                    {
                        Some((_, kind)) => value_new
                            .coerce(*kind)
                            .context("failed to coerce property value")?,
                        None => value_new,
                    };

                    *value = coerced
                }
            }
            _ => {}
        }
    }

    if node_stack.len() != 1 {
        bail!(
            "event stack was not flattened correctly (len is {})",
            node_stack.len()
        )
    }

    match node_stack.pop_back() {
        Some(e) if matches!(e, Node::Root { .. }) => Ok(e),
        _ => bail!("event stack was not created correctly (first elem is not root)"),
    }
}
