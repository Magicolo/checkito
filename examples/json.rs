use std::fmt;

use checkito::regex::Regex;

fn main() {
    use checkito::{any::Fuse, *};

    enum Node {
        Null,
        Boolean(bool),
        Number(f64),
        String(String),
        Array(Vec<Node>),
        Object(Vec<(Node, Node)>),
    }

    impl fmt::Debug for Node {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Null => write!(f, "null"),
                Self::Boolean(arg0) => write!(f, "{arg0}"),
                Self::Number(arg0) => write!(f, "{arg0}"),
                Self::String(arg0) => write!(f, r#""{arg0}""#),
                Self::Array(arg0) => f.debug_list().entries(arg0).finish(),
                Self::Object(arg0) => f
                    .debug_map()
                    .entries(arg0.iter().map(|(key, value)| (key, value)))
                    .finish(),
            }
        }
    }

    fn recurse() -> impl Generate<Item = Node> {
        lazy(node).boxed()
    }

    fn string() -> impl Generate<Item = Node> {
        "[a-zA-Z0-9]*".parse::<Regex>().unwrap().map(Node::String)
    }

    fn node() -> impl Generate<Item = Node> {
        (
            with(|| Node::Null),
            bool::generator().map(Node::Boolean),
            f64::generator().map(Node::Number),
            string(),
            recurse()
                .collect_with((..256usize).dampen())
                .map(Node::Array),
            (string(), recurse())
                .collect_with((..256usize).dampen())
                .map(Node::Object),
        )
            .any()
            .map(Fuse::fuse)
    }

    let _nodes = node().samples(100).collect::<Vec<_>>();
}
