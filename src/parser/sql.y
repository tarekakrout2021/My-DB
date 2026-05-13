%start StmtList
%avoid_insert "IDENT" "INTLIT" "STRLIT"
%%
StmtList -> Result<AST>:
    Stmt ';' StmtList { let mut ast = $3?; ast.statements.insert(0, $1?); Ok(ast) }
  | %empty            { Ok(AST { statements: vec![] }) }
  ;

Stmt -> Result<Statement>:
    CreateTableStmt { Ok($1?) }
  | CopyStmt        { Ok($1?) }
  | SelectStmt      { Ok($1?) }
  ;

SelectStmt -> Result<Statement>:
      SelectQuery { Ok(Statement::Select($1?)) }
    ;

SelectQuery -> Result<Query>:
      'SELECT' TargetList 'FROM' FromList OptWhereClause {
        Ok(Query { targets: $2?, from: $4?, r#where: $5?.unwrap_or_default() })
      }
    ;

OptWhereClause -> Result<Option<Vec<BinaryExpression>>>:
    'WHERE' PredList     { Ok(Some($2?)) }
  | %empty               { Ok(None) }
  ;


TargetList -> Result<Option<Vec<ColId>>>:
      '*'              { Ok(Some(vec![])) }
    | 'COUNT(*)'       { Ok(None) }
    | ColumnIdList     { Ok(Some($1?)) }
    ;

ColumnIdList -> Result<Vec<ColId>>:
      ColumnIdList ',' ColumnId { let mut v = $1?; v.push($3?); Ok(v) }
    | ColumnId                  { Ok(vec![$1?]) }
    ;

ColumnId -> Result<ColId>:
      'IDENT'                     { Ok(ColId { table: String::new(), column: $lexer.span_str($1?.span()).to_string() }) }
    | 'IDENT' '.' 'IDENT'           { Ok(ColId { table: $lexer.span_str($1?.span()).to_string(), column: $lexer.span_str($3?.span()).to_string() }) }
    ;

FromList -> Result<Vec<TableFactor>>:
      FromList ',' SelectTableName    { let mut v = $1?; v.push($3?); Ok(v) }
    | SelectTableName                 { Ok(vec![$1?]) }
    ;

SelectTableName -> Result<TableFactor>:
      'IDENT' 'IDENT' { Ok(TableFactor::Table(TableRef { table: $lexer.span_str($1?.span()).to_string(), alias: $lexer.span_str($2?.span()).to_string() })) }
    | 'IDENT' { Ok(TableFactor::Table(TableRef { table: $lexer.span_str($1?.span()).to_string(), alias: String::new() }))}
    | '(' SelectQuery ')' 'IDENT' { /* derived table with alias */
        Ok(TableFactor::Derived { query: Box::new($2?), alias: $lexer.span_str($4?.span()).to_string() })
      }
    | '(' SelectQuery ')' { /* derived table without alias - allow but alias will be empty */
        Ok(TableFactor::Derived { query: Box::new($2?), alias: String::new() })
      }
    ;

PredList -> Result<Vec<BinaryExpression>>:
      PredList 'AND' EqPred     { let mut v = $1?; v.push($3?); Ok(v) }
    | EqPred                    { Ok(vec![$1?]) }
    ;

EqPred -> Result<BinaryExpression>:
      ColumnId '=' ColOrConst {
        Ok(BinaryExpression {
          l: Expression::ColumnRef($1?),
          r: $3?,
          op: BinaryOperator::Eq,
        })
      }
    | ColumnId '>=' ColOrConst {
      Ok(BinaryExpression {
	l: Expression::ColumnRef($1?),
	r: $3?,
	op: BinaryOperator::Geq,
      })
    }
    | ColumnId '<=' ColOrConst {
          Ok(BinaryExpression {
    	l: Expression::ColumnRef($1?),
    	r: $3?,
    	op: BinaryOperator::Leq,
          })
    }
    | ColumnId '!=' ColOrConst {
      Ok(BinaryExpression {
	l: Expression::ColumnRef($1?),
	r: $3?,
	op: BinaryOperator::Neq,
      })
    }
    ;
ColOrConst -> Result<Expression>:
      ColumnId                  { Ok(Expression::ColumnRef($1?)) }
    | StringLit                    { Ok(Expression::String($1?)) }
    | IntLit                    { Ok(Expression::String(format!("{}", $1?))) }
    | '(' SelectQuery ')'       { Ok(Expression::Subquery(Box::new($2?))) }
    ;


CreateTableStmt -> Result<Statement>:
    'CREATE' 'TABLE' TableName '(' ColumnDefList OptTableConstraints ')' {
        let columns = $5?;
        let primary_key = $6?.unwrap_or_default();
        Ok(Statement::CreateTable(CreateTable {
            table: Table { id: $3?, columns, primary_key }
        }))
    }
  ;

TableName -> Result<String>:
    Ident { Ok($1?) }
  ;

ColumnDefList -> Result<Vec<Column>>:
    %empty { Ok(vec![]) }
  | ColumnDef {
        let col = $1?;
        Ok(vec![col])
    }
  | ColumnDefList ',' ColumnDef {
        let mut cols = $1?;
        cols.push($3?);
        Ok(cols)
    }
  ;

ColumnDef -> Result<Column>:
    Ident DataType OptNull {
        Ok(Column { id: $1?, r#type: $2?, not_null: $3? })
    }
  ;

OptNull -> Result<bool>:
   %empty {Ok(false)}
  | 'NOT' 'NULL' {Ok(true)}
  ;

OptTableConstraints -> Result<Option<Vec<String>>>:
    ',' TableConstraints { Ok(Some($2?)) }
  | %empty               { Ok(None) }
  ;

TableConstraints -> Result<Vec<String>>:
    'PRIMARY' 'KEY' '(' IdentList ')' { Ok($4?) }
  ;

DataType -> Result<crate::types::Type>:
    'INTEGER'                         { Ok(crate::types::Type::new_integer()) }
  | 'TIMESTAMP'                       { Ok(crate::types::Type::new_timestamp()) }
  | 'NUMERIC' '(' IntLit ',' IntLit ')' {
        let p: u32  = $3? as u32;
        let s_u32: u32 = $5? as u32;
        let s: u16 = u16::try_from(s_u32)
            .map_err(|_| anyhow!("NUMERIC scale must fit into u16"))?;
        Ok(crate::types::Type::new_numeric(p, s))
    }
  | 'CHAR' '(' IntLit ')'             { Ok(crate::types::Type::new_char($3? as u32)) }
  | 'VARCHAR' '(' IntLit ')'          { Ok(crate::types::Type::new_varchar($3? as u32)) }
  ;

CopyStmt -> Result<Statement>:
    'COPY' Ident 'FROM' StringLit {
        Ok(Statement::CopyTable(CopyTable {
            table: $2?, file: $4?, delimiter: "|".to_string()
        }))
    }
  | 'COPY' Ident 'FROM' StringLit 'DELIMITER' StringLit {
        Ok(Statement::CopyTable(CopyTable {
            table: $2?, file: $4?, delimiter: $6?
        }))
    }
  ;

IdentList -> Result<Vec<String>>:
    Ident { Ok(vec![$1?]) }
  | IdentList ',' Ident { let mut v = $1?; v.push($3?); Ok(v) }
  ;

Ident -> Result<String>:
    'IDENT' {
        let s = $lexer.span_str($1?.span());
        Ok(s.to_string())
    }
  | 'QUOTED_IDENT' {
        let s = $lexer.span_str($1?.span());
        let inner = &s[1..s.len()-1];
        Ok(inner.to_string())
    }
  ;

StringLit -> Result<String>:
    'STRLIT' {
        let s = $lexer.span_str($1?.span());
        // strip surrounding single quotes
        let inner = &s[1..s.len()-1];
        Ok(inner.to_string())
    }
  ;

IntLit -> Result<u64>:
    'INTLIT' {
        let s = $lexer.span_str($1?.span());
        Ok(s.parse::<u64>()?)
    }
  ;

%%

use anyhow::{Result, anyhow};
use crate::parser::ast::*;
use crate::parser::expr::*;
