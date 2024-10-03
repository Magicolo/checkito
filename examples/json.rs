#![cfg(feature = "regex")]
use checkito::*;
use core::fmt;

/// Defines json's structures.
enum Node {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<Node>),
    Object(Vec<(Node, Node)>),
}

impl Node {
    /// Computes the number of nodes in a node tree.
    pub fn size(&self) -> usize {
        match self {
            Node::Array(nodes) => nodes.len() + nodes.iter().map(Node::size).sum::<usize>(),
            Node::Object(nodes) => {
                nodes.len()
                    + nodes
                        .iter()
                        .map(|pair| pair.0.size() + pair.1.size())
                        .sum::<usize>()
            }
            _ => 1,
        }
    }
}

impl fmt::Debug for Node {
    /// Generates a json-like format that is easier to read and more compact.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Boolean(arg0) => write!(f, "{arg0}"),
            Self::Number(arg0) => write!(f, "{arg0}"),
            Self::String(arg0) => write!(f, r#""{arg0}""#),
            Self::Array(arg0) => f.debug_list().entries(arg0).finish(),
            Self::Object(arg0) => f
                .debug_map()
                .entries(arg0.iter().map(|pair| (&pair.0, &pair.1)))
                .finish(),
        }
    }
}

/// For reusability, the json string [`Generate`] implementation is factored out
/// here.
fn string() -> impl Generate<Item = Node> {
    // This somewhat convoluted regular expression produces json-compliant strings.
    regex!(r#"([a-zA-Z0-9]|[#-~ !]|(\\[\\"/bfnrt])|(\\u([0-9a-fA-F]){4}))*"#)
        // Parse the pattern into a [`Regex`] structure which implements [`Generate`].
        // Wraps the generated [`String`] in a [`Node`].
        .map(Node::String)
}

/// The general pattern for producing [`Node`]s is to generate the inner values
/// for each enum case and map them to their corresponding [`Node`] constructor
/// by using the [`Generate::map`] combinator.
fn node() -> impl Generate<Item = Node> {
    (
        // [`with`] builds a generator based on the provided function.
        // An alternative would be to use `Same(Node::Null)`, but that would required a [`Clone`]
        // implementation for [`Node`], so the [`with`] solution is preferred.
        with(|| Node::Null),
        // Uses [`bool`]'s canonical [`Generate`] through its [`FullGenerate`] implementation.
        bool::generator().map(Node::Boolean),
        // [`number`] is a helper [`Generate`] implementation that produces non-infinite and
        // non-NaN numbers.
        number::<f64>().map(Node::Number),
        string(),
        // [`lazy`] is a helper [`Generate`] implementation that prevents from recursing
        // unconditionally (since it would blow up the stack).
        lazy(node)
            // [`Generate::collect_with`] will call the previous generator a number of time defined
            // by the provided [`Generate<Item = usize>`]. [`Generate::dampen`] is used
            // to prevent an exponential explosion of nodes by reducing the `size` of the
            // [`Generate`] it is applied to as recursion goes deeper. When the maximum
            // depth is reached (see [`Generate::dampen_with`]), the `size` is set to 0.
            .collect_with((..32usize).dampen())
            .map(Node::Array)
            // [`Generate::boxed`] is used to make the return type finite. Without it, since the
            // `impl Generate` type refers to itself through the recursive calls to
            // [`node`], the type never stabilizes.
            .boxed(),
        (string(), lazy(node))
            .collect_with((..32usize).dampen())
            .map(Node::Object)
            .boxed(),
    )
        .any()
        // To be fully general, [`Generate::any`] applied to tuples produces a value of type `Or<T1,
        // T2...>` which is an enum that represents each possible item of the tuple. Since
        // the concrete type is actually `Or<Node, Node...>`, the enum can be unified into a
        // [`Node`], which is what [`Or::into`] does.
        .map(|or| or.into())
}

fn main() {
    // Will fail as soon as a node tree holds 100 nodes or more. The shrunk node
    // tree should have exactly 100 nodes, each with their smallest value.
    let result = node().check(|node| node.size() < 100);
    dbg!(result.unwrap());
}
