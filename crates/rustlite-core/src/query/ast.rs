/// Abstract Syntax Tree (AST) node types for SQL-like queries
///
/// Defines the structure of parsed queries including SELECT, FROM, WHERE, ORDER BY, LIMIT, and JOIN.
use std::fmt;

/// A complete SQL-like query
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub select: SelectClause,
    pub from: FromClause,
    pub where_clause: Option<WhereClause>,
    pub order_by: Option<OrderByClause>,
    pub limit: Option<LimitClause>,
}

/// SELECT clause specifying columns to retrieve
#[derive(Debug, Clone, PartialEq)]
pub struct SelectClause {
    pub columns: Vec<SelectColumn>,
}

/// A column in the SELECT clause
#[derive(Debug, Clone, PartialEq)]
pub enum SelectColumn {
    /// SELECT * - all columns
    Wildcard,
    /// SELECT column_name or SELECT column_name AS alias
    Column { name: String, alias: Option<String> },
    /// SELECT COUNT(*), SUM(column), etc.
    Aggregate {
        function: AggregateFunction,
        column: Box<SelectColumn>,
        alias: Option<String>,
    },
}

/// Aggregate functions
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// FROM clause specifying tables
#[derive(Debug, Clone, PartialEq)]
pub struct FromClause {
    pub table: String,
    pub joins: Vec<Join>,
}

/// JOIN clause
#[derive(Debug, Clone, PartialEq)]
pub struct Join {
    pub join_type: JoinType,
    pub table: String,
    pub condition: Expression,
}

/// Types of joins
#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

/// WHERE clause for filtering
#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    pub condition: Expression,
}

/// Boolean expression for WHERE and JOIN conditions
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Column reference
    Column(String),
    /// Literal value
    Literal(Literal),
    /// Binary operation: column = value, column > value, etc.
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    /// Logical AND/OR
    LogicalOp {
        left: Box<Expression>,
        op: LogicalOperator,
        right: Box<Expression>,
    },
    /// NOT expression
    Not(Box<Expression>),
    /// LIKE pattern matching
    Like {
        expr: Box<Expression>,
        pattern: String,
    },
    /// IN (value1, value2, ...)
    In {
        expr: Box<Expression>,
        values: Vec<Literal>,
    },
    /// BETWEEN min AND max
    Between {
        expr: Box<Expression>,
        min: Box<Expression>,
        max: Box<Expression>,
    },
}

/// Binary comparison operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Eq, // =
    Ne, // !=
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
}

/// Logical operators
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOperator {
    And,
    Or,
}

/// Literal values in queries
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
}

/// ORDER BY clause for sorting
#[derive(Debug, Clone, PartialEq)]
pub struct OrderByClause {
    pub columns: Vec<OrderByColumn>,
}

/// A column in ORDER BY
#[derive(Debug, Clone, PartialEq)]
pub struct OrderByColumn {
    pub column: String,
    pub direction: OrderDirection,
}

/// Sort direction
#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

/// LIMIT clause for result limiting
#[derive(Debug, Clone, PartialEq)]
pub struct LimitClause {
    pub count: usize,
    pub offset: Option<usize>,
}

// Display implementations for debugging and error messages

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.select, self.from)?;
        if let Some(ref where_clause) = self.where_clause {
            write!(f, " {}", where_clause)?;
        }
        if let Some(ref order_by) = self.order_by {
            write!(f, " {}", order_by)?;
        }
        if let Some(ref limit) = self.limit {
            write!(f, " {}", limit)?;
        }
        Ok(())
    }
}

impl fmt::Display for SelectClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SELECT ")?;
        for (i, col) in self.columns.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", col)?;
        }
        Ok(())
    }
}

impl fmt::Display for SelectColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectColumn::Wildcard => write!(f, "*"),
            SelectColumn::Column { name, alias } => {
                write!(f, "{}", name)?;
                if let Some(ref alias) = alias {
                    write!(f, " AS {}", alias)?;
                }
                Ok(())
            }
            SelectColumn::Aggregate {
                function,
                column,
                alias,
            } => {
                write!(f, "{}({})", function, column)?;
                if let Some(ref alias) = alias {
                    write!(f, " AS {}", alias)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for AggregateFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregateFunction::Count => write!(f, "COUNT"),
            AggregateFunction::Sum => write!(f, "SUM"),
            AggregateFunction::Avg => write!(f, "AVG"),
            AggregateFunction::Min => write!(f, "MIN"),
            AggregateFunction::Max => write!(f, "MAX"),
        }
    }
}

impl fmt::Display for FromClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FROM {}", self.table)?;
        for join in &self.joins {
            write!(f, " {}", join)?;
        }
        Ok(())
    }
}

impl fmt::Display for Join {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} JOIN {} ON {}",
            self.join_type, self.table, self.condition
        )
    }
}

impl fmt::Display for JoinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JoinType::Inner => write!(f, "INNER"),
            JoinType::Left => write!(f, "LEFT"),
            JoinType::Right => write!(f, "RIGHT"),
            JoinType::Full => write!(f, "FULL"),
        }
    }
}

impl fmt::Display for WhereClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WHERE {}", self.condition)
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Column(name) => write!(f, "{}", name),
            Expression::Literal(lit) => write!(f, "{}", lit),
            Expression::BinaryOp { left, op, right } => {
                write!(f, "({} {} {})", left, op, right)
            }
            Expression::LogicalOp { left, op, right } => {
                write!(f, "({} {} {})", left, op, right)
            }
            Expression::Not(expr) => write!(f, "NOT ({})", expr),
            Expression::Like { expr, pattern } => write!(f, "{} LIKE '{}'", expr, pattern),
            Expression::In { expr, values } => {
                write!(f, "{} IN (", expr)?;
                for (i, val) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, ")")
            }
            Expression::Between { expr, min, max } => {
                write!(f, "{} BETWEEN {} AND {}", expr, min, max)
            }
        }
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOperator::Eq => write!(f, "="),
            BinaryOperator::Ne => write!(f, "!="),
            BinaryOperator::Lt => write!(f, "<"),
            BinaryOperator::Le => write!(f, "<="),
            BinaryOperator::Gt => write!(f, ">"),
            BinaryOperator::Ge => write!(f, ">="),
        }
    }
}

impl fmt::Display for LogicalOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicalOperator::And => write!(f, "AND"),
            LogicalOperator::Or => write!(f, "OR"),
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Integer(i) => write!(f, "{}", i),
            Literal::Float(fl) => write!(f, "{}", fl),
            Literal::String(s) => write!(f, "'{}'", s),
            Literal::Boolean(b) => write!(f, "{}", b),
            Literal::Null => write!(f, "NULL"),
        }
    }
}

impl fmt::Display for OrderByClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ORDER BY ")?;
        for (i, col) in self.columns.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", col)?;
        }
        Ok(())
    }
}

impl fmt::Display for OrderByColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.column, self.direction)
    }
}

impl fmt::Display for OrderDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderDirection::Asc => write!(f, "ASC"),
            OrderDirection::Desc => write!(f, "DESC"),
        }
    }
}

impl fmt::Display for LimitClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LIMIT {}", self.count)?;
        if let Some(offset) = self.offset {
            write!(f, " OFFSET {}", offset)?;
        }
        Ok(())
    }
}
