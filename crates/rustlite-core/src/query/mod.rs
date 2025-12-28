/// Query engine module
///
/// SQL-like query parsing, planning, and execution.
/// Abstract Syntax Tree types
#[allow(missing_docs)]
pub mod ast;
/// Query executor
#[allow(missing_docs)]
pub mod executor;
/// SQL lexer
#[allow(missing_docs)]
pub mod lexer;
/// SQL parser
#[allow(missing_docs)]
pub mod parser;
/// Query planner
#[allow(missing_docs)]
pub mod planner;

// Re-export main types
pub use ast::*;
pub use executor::{Column, ExecutionContext, Executor, Row, Value};
pub use lexer::{Lexer, LexerError, Token};
pub use parser::{ParseError, Parser};
pub use planner::{IndexMetadata, PhysicalOperator, PhysicalPlan, PlanError, Planner};
