use chumsky::Parser;
use chumsky::prelude::*;
use chumsky::text::{ident, whitespace};
use grabapl::SubstMarker;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum NodeType {
    Object,
    String,
    Integer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    ExpectParameterNode(SubstMarker, NodeType),
}

fn command<'a>() -> impl Parser<'a, &'a str, Command, extra::Err<Rich<'a, char>>> {
    choice([just("expect_parameter_node")
        .ignore_then(
            whitespace()
                .at_least(1)
                .ignore_then(ident().map(SubstMarker::from))
                .then_ignore(whitespace().at_least(1))
                .then(choice((
                    just("object").to(NodeType::Object),
                    just("string").to(NodeType::String),
                    just("integer").to(NodeType::Integer),
                ))),
        )
        .map(|(s, t)| Command::ExpectParameterNode(s, t))])
}

fn main() {
    // loop over input lines
    for line in std::io::stdin().lines() {
        let line = line.expect("Failed to read line");
        println!("Parsing line: {}", line);
        let result = command().parse(line.as_str());
        let cmd = result.unwrap();
        println!("{:?}", cmd);
    }
}
