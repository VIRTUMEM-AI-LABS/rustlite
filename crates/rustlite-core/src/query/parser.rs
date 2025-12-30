/// Parser for SQL-like queries
///
/// Converts a stream of tokens into an Abstract Syntax Tree (AST).
use super::ast::*;
use super::lexer::{Lexer, LexerError, Token};
use std::fmt;

/// Parser for SQL-like queries
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a new parser from SQL text
    pub fn new(input: &str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().map_err(ParseError::LexerError)?;
        Ok(Self {
            tokens,
            position: 0,
        })
    }

    /// Parse the query into an AST
    pub fn parse(&mut self) -> Result<Query, ParseError> {
        let select = self.parse_select()?;
        let from = self.parse_from()?;
        let where_clause = self.parse_where()?;
        let group_by = self.parse_group_by()?;
        let having = self.parse_having()?;
        let order_by = self.parse_order_by()?;
        let limit = self.parse_limit()?;

        self.expect_token(Token::Eof)?;

        Ok(Query {
            select,
            from,
            where_clause,
            group_by,
            having,
            order_by,
            limit,
        })
    }

    fn parse_select(&mut self) -> Result<SelectClause, ParseError> {
        self.expect_token(Token::Select)?;

        let mut columns = Vec::new();

        loop {
            if self.current_token() == &Token::Asterisk {
                self.advance();
                columns.push(SelectColumn::Wildcard);
            } else if matches!(
                self.current_token(),
                Token::Count | Token::Sum | Token::Avg | Token::Min | Token::Max
            ) {
                // Aggregate function
                let function = match self.current_token() {
                    Token::Count => AggregateFunction::Count,
                    Token::Sum => AggregateFunction::Sum,
                    Token::Avg => AggregateFunction::Avg,
                    Token::Min => AggregateFunction::Min,
                    Token::Max => AggregateFunction::Max,
                    _ => unreachable!(),
                };
                self.advance();

                self.expect_token(Token::LeftParen)?;

                let column = if self.current_token() == &Token::Asterisk {
                    self.advance();
                    Box::new(SelectColumn::Wildcard)
                } else if let Token::Identifier(name) = self.current_token().clone() {
                    self.advance();
                    Box::new(SelectColumn::Column { name, alias: None })
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "column name or *".to_string(),
                        found: self.current_token().clone(),
                    });
                };

                self.expect_token(Token::RightParen)?;

                let alias = if self.current_token() == &Token::As {
                    self.advance();
                    if let Token::Identifier(name) = self.current_token().clone() {
                        self.advance();
                        Some(name)
                    } else {
                        None
                    }
                } else {
                    None
                };

                columns.push(SelectColumn::Aggregate {
                    function,
                    column,
                    alias,
                });
            } else if let Token::Identifier(name) = self.current_token().clone() {
                self.advance();

                let alias = if self.current_token() == &Token::As {
                    self.advance();
                    if let Token::Identifier(alias_name) = self.current_token().clone() {
                        self.advance();
                        Some(alias_name)
                    } else {
                        None
                    }
                } else {
                    None
                };

                columns.push(SelectColumn::Column { name, alias });
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "column name or *".to_string(),
                    found: self.current_token().clone(),
                });
            }

            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        if columns.is_empty() {
            return Err(ParseError::EmptySelectList);
        }

        Ok(SelectClause { columns })
    }

    fn parse_from(&mut self) -> Result<FromClause, ParseError> {
        self.expect_token(Token::From)?;

        let table = if let Token::Identifier(name) = self.current_token().clone() {
            self.advance();
            name
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "table name".to_string(),
                found: self.current_token().clone(),
            });
        };

        let mut joins = Vec::new();

        // Parse JOINs
        while matches!(
            self.current_token(),
            Token::Inner | Token::Left | Token::Right | Token::Full | Token::Join
        ) {
            let join_type = match self.current_token() {
                Token::Inner => {
                    self.advance();
                    self.expect_token(Token::Join)?;
                    JoinType::Inner
                }
                Token::Left => {
                    self.advance();
                    self.expect_token(Token::Join)?;
                    JoinType::Left
                }
                Token::Right => {
                    self.advance();
                    self.expect_token(Token::Join)?;
                    JoinType::Right
                }
                Token::Full => {
                    self.advance();
                    self.expect_token(Token::Join)?;
                    JoinType::Full
                }
                Token::Join => {
                    self.advance();
                    JoinType::Inner // Default to INNER JOIN
                }
                _ => break,
            };

            let join_table = if let Token::Identifier(name) = self.current_token().clone() {
                self.advance();
                name
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "table name".to_string(),
                    found: self.current_token().clone(),
                });
            };

            self.expect_token(Token::On)?;
            let condition = self.parse_expression()?;

            joins.push(Join {
                join_type,
                table: join_table,
                condition,
            });
        }

        Ok(FromClause { table, joins })
    }

    fn parse_where(&mut self) -> Result<Option<WhereClause>, ParseError> {
        if self.current_token() != &Token::Where {
            return Ok(None);
        }

        self.advance();
        let condition = self.parse_expression()?;

        Ok(Some(WhereClause { condition }))
    }

    fn parse_group_by(&mut self) -> Result<Option<GroupByClause>, ParseError> {
        if self.current_token() != &Token::Group {
            return Ok(None);
        }

        self.advance();
        self.expect_token(Token::By)?;

        let mut columns = Vec::new();

        loop {
            if let Token::Identifier(name) = self.current_token().clone() {
                self.advance();
                columns.push(name);

                if self.current_token() == &Token::Comma {
                    self.advance();
                    continue;
                } else {
                    break;
                }
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "column name".to_string(),
                    found: self.current_token().clone(),
                });
            }
        }

        if columns.is_empty() {
            return Err(ParseError::UnexpectedToken {
                expected: "at least one column for GROUP BY".to_string(),
                found: self.current_token().clone(),
            });
        }

        Ok(Some(GroupByClause { columns }))
    }

    fn parse_having(&mut self) -> Result<Option<HavingClause>, ParseError> {
        if self.current_token() != &Token::Having {
            return Ok(None);
        }

        self.advance();
        let condition = self.parse_expression()?;

        Ok(Some(HavingClause { condition }))
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_logical_and()?;

        while self.current_token() == &Token::Or {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expression::LogicalOp {
                left: Box::new(left),
                op: LogicalOperator::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_not()?;

        while self.current_token() == &Token::And {
            self.advance();
            let right = self.parse_not()?;
            left = Expression::LogicalOp {
                left: Box::new(left),
                op: LogicalOperator::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expression, ParseError> {
        if self.current_token() == &Token::Not {
            self.advance();
            let expr = self.parse_comparison()?;
            return Ok(Expression::Not(Box::new(expr)));
        }

        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_primary()?;

        // Handle LIKE
        if self.current_token() == &Token::Like {
            self.advance();
            if let Token::String(pattern) = self.current_token().clone() {
                self.advance();
                return Ok(Expression::Like {
                    expr: Box::new(left),
                    pattern,
                });
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "string pattern".to_string(),
                    found: self.current_token().clone(),
                });
            }
        }

        // Handle IN
        if self.current_token() == &Token::In {
            self.advance();
            self.expect_token(Token::LeftParen)?;

            let mut values = Vec::new();
            loop {
                let value = self.parse_literal()?;
                values.push(value);

                if self.current_token() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }

            self.expect_token(Token::RightParen)?;

            return Ok(Expression::In {
                expr: Box::new(left),
                values,
            });
        }

        // Handle BETWEEN
        if self.current_token() == &Token::Between {
            self.advance();
            let min = self.parse_primary()?;
            self.expect_token(Token::And)?;
            let max = self.parse_primary()?;

            return Ok(Expression::Between {
                expr: Box::new(left),
                min: Box::new(min),
                max: Box::new(max),
            });
        }

        // Handle comparison operators
        let op = match self.current_token() {
            Token::Eq => BinaryOperator::Eq,
            Token::Ne => BinaryOperator::Ne,
            Token::Lt => BinaryOperator::Lt,
            Token::Le => BinaryOperator::Le,
            Token::Gt => BinaryOperator::Gt,
            Token::Ge => BinaryOperator::Ge,
            _ => return Ok(left),
        };

        self.advance();
        let right = self.parse_primary()?;

        Ok(Expression::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match self.current_token().clone() {
            Token::Identifier(name) => {
                self.advance();
                Ok(Expression::Column(name))
            }
            Token::Integer(i) => {
                self.advance();
                Ok(Expression::Literal(Literal::Integer(i)))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expression::Literal(Literal::Float(f)))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expression::Literal(Literal::String(s)))
            }
            Token::Boolean(b) => {
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(b)))
            }
            Token::Null => {
                self.advance();
                Ok(Expression::Literal(Literal::Null))
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_token(Token::RightParen)?;
                Ok(expr)
            }
            token => Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: token,
            }),
        }
    }

    fn parse_literal(&mut self) -> Result<Literal, ParseError> {
        match self.current_token().clone() {
            Token::Integer(i) => {
                self.advance();
                Ok(Literal::Integer(i))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Literal::Float(f))
            }
            Token::String(s) => {
                self.advance();
                Ok(Literal::String(s))
            }
            Token::Boolean(b) => {
                self.advance();
                Ok(Literal::Boolean(b))
            }
            Token::Null => {
                self.advance();
                Ok(Literal::Null)
            }
            token => Err(ParseError::UnexpectedToken {
                expected: "literal value".to_string(),
                found: token,
            }),
        }
    }

    fn parse_order_by(&mut self) -> Result<Option<OrderByClause>, ParseError> {
        if self.current_token() != &Token::OrderBy {
            return Ok(None);
        }

        self.advance();

        let mut columns = Vec::new();

        loop {
            let column = if let Token::Identifier(name) = self.current_token().clone() {
                self.advance();
                name
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "column name".to_string(),
                    found: self.current_token().clone(),
                });
            };

            let direction = if self.current_token() == &Token::Desc {
                self.advance();
                OrderDirection::Desc
            } else {
                if self.current_token() == &Token::Asc {
                    self.advance();
                }
                OrderDirection::Asc
            };

            columns.push(OrderByColumn { column, direction });

            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Some(OrderByClause { columns }))
    }

    fn parse_limit(&mut self) -> Result<Option<LimitClause>, ParseError> {
        if self.current_token() != &Token::Limit {
            return Ok(None);
        }

        self.advance();

        let count = if let Token::Integer(n) = self.current_token() {
            if *n < 0 {
                return Err(ParseError::InvalidLimitValue(*n));
            }
            let count = *n as usize;
            self.advance();
            count
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "integer".to_string(),
                found: self.current_token().clone(),
            });
        };

        let offset = if self.current_token() == &Token::Offset {
            self.advance();
            if let Token::Integer(n) = self.current_token() {
                if *n < 0 {
                    return Err(ParseError::InvalidOffsetValue(*n));
                }
                let offset = *n as usize;
                self.advance();
                Some(offset)
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "integer".to_string(),
                    found: self.current_token().clone(),
                });
            }
        } else {
            None
        };

        Ok(Some(LimitClause { count, offset }))
    }

    fn current_token(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }
    }

    fn expect_token(&mut self, expected: Token) -> Result<(), ParseError> {
        if self.current_token() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{}", expected),
                found: self.current_token().clone(),
            })
        }
    }
}

/// Parser errors
#[derive(Debug, Clone)]
pub enum ParseError {
    LexerError(LexerError),
    UnexpectedToken { expected: String, found: Token },
    EmptySelectList,
    InvalidLimitValue(i64),
    InvalidOffsetValue(i64),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::LexerError(e) => write!(f, "Lexer error: {}", e),
            ParseError::UnexpectedToken { expected, found } => {
                write!(f, "Expected {}, found {}", expected, found)
            }
            ParseError::EmptySelectList => write!(f, "SELECT list cannot be empty"),
            ParseError::InvalidLimitValue(n) => {
                write!(f, "Invalid LIMIT value: {} (must be non-negative)", n)
            }
            ParseError::InvalidOffsetValue(n) => {
                write!(f, "Invalid OFFSET value: {} (must be non-negative)", n)
            }
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let mut parser = Parser::new("SELECT * FROM users").unwrap();
        let query = parser.parse().unwrap();

        assert_eq!(query.select.columns.len(), 1);
        assert!(matches!(query.select.columns[0], SelectColumn::Wildcard));
        assert_eq!(query.from.table, "users");
    }

    #[test]
    fn test_select_with_columns() {
        let mut parser = Parser::new("SELECT name, age FROM users").unwrap();
        let query = parser.parse().unwrap();

        assert_eq!(query.select.columns.len(), 2);
    }

    #[test]
    fn test_select_with_where() {
        let mut parser = Parser::new("SELECT * FROM users WHERE age > 18").unwrap();
        let query = parser.parse().unwrap();

        assert!(query.where_clause.is_some());
    }

    #[test]
    fn test_select_with_order_by() {
        let mut parser = Parser::new("SELECT * FROM users ORDER BY name ASC").unwrap();
        let query = parser.parse().unwrap();

        assert!(query.order_by.is_some());
        let order_by = query.order_by.unwrap();
        assert_eq!(order_by.columns.len(), 1);
        assert_eq!(order_by.columns[0].column, "name");
        assert_eq!(order_by.columns[0].direction, OrderDirection::Asc);
    }

    #[test]
    fn test_select_with_limit() {
        let mut parser = Parser::new("SELECT * FROM users LIMIT 10").unwrap();
        let query = parser.parse().unwrap();

        assert!(query.limit.is_some());
        let limit = query.limit.unwrap();
        assert_eq!(limit.count, 10);
        assert_eq!(limit.offset, None);
    }

    #[test]
    fn test_select_with_limit_offset() {
        let mut parser = Parser::new("SELECT * FROM users LIMIT 10 OFFSET 5").unwrap();
        let query = parser.parse().unwrap();

        let limit = query.limit.unwrap();
        assert_eq!(limit.count, 10);
        assert_eq!(limit.offset, Some(5));
    }

    #[test]
    fn test_complex_where() {
        let mut parser =
            Parser::new("SELECT * FROM users WHERE age > 18 AND name = 'John'").unwrap();
        let query = parser.parse().unwrap();

        assert!(query.where_clause.is_some());
    }

    #[test]
    fn test_aggregate_function() {
        let mut parser = Parser::new("SELECT COUNT(*) FROM users").unwrap();
        let query = parser.parse().unwrap();

        assert_eq!(query.select.columns.len(), 1);
        assert!(matches!(
            query.select.columns[0],
            SelectColumn::Aggregate { .. }
        ));
    }

    #[test]
    fn test_join() {
        let mut parser =
            Parser::new("SELECT * FROM users INNER JOIN orders ON users.id = orders.user_id")
                .unwrap();
        let query = parser.parse().unwrap();

        assert_eq!(query.from.joins.len(), 1);
        assert_eq!(query.from.joins[0].join_type, JoinType::Inner);
        assert_eq!(query.from.joins[0].table, "orders");
    }
}
