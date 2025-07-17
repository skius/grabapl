//! This is an entire parser and interpreter for a dynamically-typed Rust-like expression-oriented
//! programming language. See `sample.nrs` for sample source code.
//! Run it with the following command:
//! cargo run --features="label" --example nano_rust -- examples/sample.nrs

use ariadne::{Color, Label, Report, ReportKind, sources};
use chumsky::{input::ValueInput, prelude::*};
use std::{collections::HashMap, env, fmt, fs};

use syntax::*;

fn main() {
    // println!("{:?}", ascii_ident_fixed::<&str, extra::Err<Rich<char>>>().map(|x: &str| x).parse("field1").unwrap());

    let filename = env::args().nth(1).expect("Expected file argument");
    let src = fs::read_to_string(&filename).expect("Failed to read file");

    println!("Source: {src}");

    let (tokens, mut errs) = lexer().parse(src.as_str()).into_output_errors();

    println!("Tokens: {tokens:?}");

    let parse_errs = if let Some(tokens) = &tokens {
        let (ast, parse_errs) = program_parser::<_, MyCustomSyntax>()
            .map_with(|ast, e| (ast, e.span()))
            .parse(
                tokens
                    .as_slice()
                    .map((src.len()..src.len()).into(), |(t, s)| (t, s)),
            )
            .into_output_errors();

        if let Some((funcs, file_span)) = ast.filter(|_| errs.len() + parse_errs.len() == 0) {
            println!("Parsed: {funcs:?}");
        }

        parse_errs
    } else {
        Vec::new()
    };

    errs.into_iter()
        .map(|e| e.map_token(|c| c.to_string()))
        .chain(
            parse_errs
                .into_iter()
                .map(|e| e.map_token(|tok| tok.to_string())),
        )
        .for_each(|e| {
            Report::build(ReportKind::Error, (filename.clone(), e.span().into_range()))
                .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                .with_message(e.to_string())
                .with_label(
                    Label::new((filename.clone(), e.span().into_range()))
                        .with_message(e.reason().to_string())
                        .with_color(Color::Red),
                )
                .with_labels(e.contexts().map(|(label, span)| {
                    Label::new((filename.clone(), span.into_range()))
                        .with_message(format!("while parsing this {label}"))
                        .with_color(Color::Yellow)
                }))
                .finish()
                .print(sources([(filename.clone(), src.clone())]))
                .unwrap()
        });
}

// use syntax::minirust::*;
//
// fn main() {
//     let filename = env::args().nth(1).expect("Expected file argument");
//     let src = fs::read_to_string(&filename).expect("Failed to read file");
//
//     let (tokens, mut errs) = lexer().parse(src.as_str()).into_output_errors();
//
//     let parse_errs = if let Some(tokens) = &tokens {
//         let (ast, parse_errs) = funcs_parser()
//             .map_with(|ast, e| (ast, e.span()))
//             .parse(
//                 tokens
//                     .as_slice()
//                     .map((src.len()..src.len()).into(), |(t, s)| (t, s)),
//             )
//             .into_output_errors();
//
//         if let Some((funcs, file_span)) = ast.filter(|_| errs.len() + parse_errs.len() == 0) {
//             if let Some(main) = funcs.get("main") {
//                 if !main.args.is_empty() {
//                     errs.push(Rich::custom(
//                         main.span,
//                         "The main function cannot have arguments".to_string(),
//                     ))
//                 } else {
//                     match eval_expr(&main.body, &funcs, &mut Vec::new()) {
//                         Ok(val) => println!("Return value: {val}"),
//                         Err(e) => errs.push(Rich::custom(e.span, e.msg)),
//                     }
//                 }
//             } else {
//                 errs.push(Rich::custom(
//                     file_span,
//                     "Programs need a main function but none was found".to_string(),
//                 ));
//             }
//         }
//
//         parse_errs
//     } else {
//         Vec::new()
//     };
//
//     errs.into_iter()
//         .map(|e| e.map_token(|c| c.to_string()))
//         .chain(
//             parse_errs
//                 .into_iter()
//                 .map(|e| e.map_token(|tok| tok.to_string())),
//         )
//         .for_each(|e| {
//             Report::build(ReportKind::Error, (filename.clone(), e.span().into_range()))
//                 .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
//                 .with_message(e.to_string())
//                 .with_label(
//                     Label::new((filename.clone(), e.span().into_range()))
//                         .with_message(e.reason().to_string())
//                         .with_color(Color::Red),
//                 )
//                 .with_labels(e.contexts().map(|(label, span)| {
//                     Label::new((filename.clone(), span.into_range()))
//                         .with_message(format!("while parsing this {label}"))
//                         .with_color(Color::Yellow)
//                 }))
//                 .finish()
//                 .print(sources([(filename.clone(), src.clone())]))
//                 .unwrap()
//         });
// }
