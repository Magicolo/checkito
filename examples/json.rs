use checkito::regex::Regex;

fn main() {
    use checkito::{any::Fuse, *};

    #[derive(Debug)]
    enum Node {
        Null,
        Boolean(bool),
        Number(f64),
        String(String),
        Array(Vec<Node>),
        Object(Vec<(Node, Node)>),
    }

    fn string() -> impl Generate<Item = Node> {
        "[a-zA-Z0-9]"
            .parse::<Regex>()
            .unwrap()
            .collect()
            .map(Node::String)
    }

    fn node() -> impl Generate<Item = Node> {
        (
            with(|| Node::Null),
            bool::generator().map(Node::Boolean),
            f64::generator().map(Node::Number),
            string(),
            lazy(node).collect_with(..4usize).map(Node::Array).boxed(),
            (string(), lazy(node))
                .collect_with(0..4usize)
                .map(Node::Object)
                .boxed(),
        )
            .any()
            .map(Fuse::fuse)
    }

    let nodes = dbg!(node().samples(100).collect::<Vec<_>>());
}
