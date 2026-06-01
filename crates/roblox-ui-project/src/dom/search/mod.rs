/*!
    Implementation of the Roblox Studio explorer
    [search methods](https://create.roblox.com/docs/studio/explorer#search-methods).

    The grammar, matched against the live Studio behaviour, consists of:

    - Name search: a bare string matches instances whose `Name` contains it,
      case-insensitively.
    - `is:Class`: instances of `Class` or any subclass (the `IsA` relation).
    - `tag:Tag`: instances with the given `CollectionService` tag (quote tags
      with spaces, e.g. `tag:"Light Source"`).
    - Property comparisons `Prop OP Value` for `OP` in `= == ~= > < >= <=`:
      string compares are partial, case- and space-insensitive; numbers compare
      numerically; sub-fields (`Position.X`, `Color.R`) and quoted composite
      units (`Size > "20, 5, 20"`) are supported, as are enums by item name
      (`Material == plas`). Comparisons see the *effective* value, falling back
      to the class default.
    - Ancestry paths: `Parent.Child` (dotted), with `*` (one level) and `**`
      (any number of descendant levels).
    - Boolean combinators: `and` (explicit or implicit between adjacent terms),
      `or`, and `(...)` grouping.

    The engine is split into the [`lexer`], [`parser`] (producing the [`ast`]),
    and [`matcher`]. It operates purely on the [`Dom`] - there is no I/O - so each
    stage can be unit-tested in isolation.
*/

use rbx_dom_weak::types::Ref;

use super::Dom;

mod ast;
mod lexer;
mod matcher;
mod parser;

/**
    Runs a Studio explorer search over the descendants of the [`Dom`] root,
    returning matches in preorder (tree) depth-first order, capped at `limit`.

    The walk stops early once `limit` matches have been collected. A blank or
    unparseable query matches nothing.
*/
pub(crate) fn search(dom: &Dom, query: &str, limit: Option<usize>) -> Vec<Ref> {
    let Some(query) = parser::parse(query) else {
        return Vec::new();
    };

    let mut results = Vec::new();
    if let Some(root) = dom.get_root_id() {
        collect(dom, root, &query, limit, &mut results);
    }
    results
}

fn collect(dom: &Dom, id: Ref, query: &ast::Query, limit: Option<usize>, results: &mut Vec<Ref>) {
    for child in dom.children(id) {
        if limit.is_some_and(|l| results.len() >= l) {
            return;
        }
        if matcher::matches(dom, *child, query) {
            results.push(*child);
        }
        collect(dom, *child, query, limit, results);
    }
}
