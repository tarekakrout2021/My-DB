use cfgrammar::yacc::YaccKind;
use lrlex::CTLexerBuilder;

fn main() {
    // lexer and parser
    CTLexerBuilder::new()
        .lrpar_config(|ctp| {
            ctp.yacckind(YaccKind::Grmtools)
                .grammar_in_src_dir("parser/sql.y")
                .unwrap()
        })
        .lexer_in_src_dir("parser/sql.l")
        .unwrap()
        .case_insensitive(true)
        .build()
        .unwrap();
}
