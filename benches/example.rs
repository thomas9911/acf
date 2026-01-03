use bencher::{Bencher, black_box};
use acf::tokenized_to_config;
use acf::parser::tokenize_ast;

use acf::tokenizer::{tokenize, parse};

const DATA: &str = r#"
    config1={value: 1, default: 12, yes: true},
    config2={DEFAULT: "testing", extra: "extra \"quotes\""},
    config3={false, 123, 1.23}
    "#;

fn a(bench: &mut Bencher) {
    bench.iter(|| {
        let data = black_box(DATA);
        let tokens = tokenize_ast(data).unwrap();
        tokenized_to_config(data, tokens)
    })
}

fn b(bench: &mut Bencher) {
    bench.iter(|| {
        let data = black_box(DATA);
        let lexer = tokenize(data);
        parse(lexer).unwrap()
    });
}

bencher::benchmark_group!(benches, a, b);
bencher::benchmark_main!(benches);
