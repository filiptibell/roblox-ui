/*!
    Abstract syntax tree for the Studio explorer search grammar.

    A [`Query`] is a boolean tree (`and` / `or`) over leaf filters: a name
    substring, an `is:` class test, a `tag:` test, a [`Property`] comparison, or
    a dotted [`Ancestry`] path of [`Seg`]ments.

    [`Property`]: Query::Property
    [`Ancestry`]: Query::Ancestry
*/

/**
    A comparison operator. Both `=` and `==` map to [`Op::Eq`].
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

/**
    A single segment of a dotted ancestry path.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Seg {
    Name(String),
    /// `*` - exactly one level.
    AnyOne,
    /// `**` - one or more levels (i.e. strict descendants).
    AnyDepth,
}

/**
    A parsed search query.

    The boolean variants nest to encode precedence (`or` over `and`); the leaf
    variants each correspond to one piece of the search grammar.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Query {
    And(Vec<Query>),
    Or(Vec<Query>),
    /**
        A case-insensitive substring match against `Instance.Name`.
    */
    Name(String),
    /**
        `is:Class` - matches the class or any subclass (the `IsA` relation).
    */
    Is(String),
    /**
        `tag:Tag` - matches a `CollectionService` tag.
    */
    Tag(String),
    /**
        `Prop OP value`, where `path` is the property name plus any sub-fields
        (e.g. `["Position", "X"]`).
    */
    Property {
        path: Vec<String>,
        op: Op,
        value: String,
    },
    /**
        A dotted ancestry path, e.g. `workspace.Model` or `Cart.*.Trim`.
    */
    Ancestry(Vec<Seg>),
}
